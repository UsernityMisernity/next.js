use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use next_transform_dynamic::{next_dynamic, NextDynamicMode};
use swc_core::{
    common::{util::take::Take, FileName},
    ecma::{
        ast::{Module, Program},
        visit::FoldWith,
    },
};
use turbopack_binding::{
    turbo::tasks_fs::FileSystemPathVc,
    turbopack::{
        ecmascript::{
            CustomTransformer, EcmascriptInputTransform, EcmascriptInputTransformsVc,
            TransformContext, TransformPluginVc,
        },
        turbopack::module_options::{ModuleRule, ModuleRuleEffect},
    },
};

use super::module_rule_match_js_no_url;
use crate::mode::NextMode;

/// Returns a rule which applies the Next.js dynamic transform.
pub async fn get_next_dynamic_transform_rule(
    is_server: bool,
    is_server_components: bool,
    pages_dir: Option<FileSystemPathVc>,
    mode: NextMode,
) -> Result<ModuleRule> {
    let dynamic_transform =
        EcmascriptInputTransform::Plugin(TransformPluginVc::cell(Box::new(NextJsDynamic {
            is_server,
            is_server_components,
            pages_dir: match pages_dir {
                None => None,
                Some(path) => Some(path.await?.path.clone().into()),
            },
            mode,
        })));
    Ok(ModuleRule::new(
        module_rule_match_js_no_url(),
        vec![ModuleRuleEffect::AddEcmascriptTransforms(
            EcmascriptInputTransformsVc::cell(vec![dynamic_transform]),
        )],
    ))
}

#[derive(Debug)]
struct NextJsDynamic {
    is_server: bool,
    is_server_components: bool,
    pages_dir: Option<PathBuf>,
    mode: NextMode,
}

#[async_trait]
impl CustomTransformer for NextJsDynamic {
    async fn transform(&self, program: &mut Program, ctx: &TransformContext<'_>) -> Result<()> {
        let p = std::mem::replace(program, Program::Module(Module::dummy()));
        *program = p.fold_with(&mut next_dynamic(
            match self.mode {
                NextMode::Development => true,
                NextMode::Build => false,
            },
            self.is_server,
            self.is_server_components,
            NextDynamicMode::Turbopack {
                dynamic_transition_name: match self.mode {
                    NextMode::Development => "next-client-chunks".to_string(),
                    NextMode::Build => "next-dynamic".to_string(),
                },
            },
            FileName::Real(ctx.file_path_str.into()),
            self.pages_dir.clone(),
        ));

        Ok(())
    }
}
