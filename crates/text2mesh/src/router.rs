//! Lattice Router (design §5). Pure. Cadre compose is S7 refuse-if-absent.

use crate::classify::classify;
use crate::types::{JobSubmit, PromptClass, Route};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteDecision {
    Image,
    Analytic,
    ViewContract,
    Native,
}

pub fn route_job(spec: &JobSubmit) -> RouteDecision {
    match spec.route {
        Route::Analytic => RouteDecision::Analytic,
        Route::ViewContract => RouteDecision::ViewContract,
        Route::Native => RouteDecision::Native,
        Route::Auto => {
            let has_image = spec
                .image_path
                .as_deref()
                .map(str::trim)
                .is_some_and(|s| !s.is_empty());
            if has_image {
                return RouteDecision::Image;
            }
            let prompt = spec.prompt.as_deref().unwrap_or("");
            let class = classify(prompt);
            if class == PromptClass::Analytic && !spec.allow_neural_cad {
                return RouteDecision::Analytic;
            }
            if spec.allow_native_text {
                return RouteDecision::Native;
            }
            RouteDecision::ViewContract
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::JobSubmit;

    #[test]
    fn fox_goes_view_contract() {
        let spec = JobSubmit {
            prompt: Some("a red fox wearing a yellow raincoat".into()),
            ..JobSubmit::default()
        };
        assert_eq!(route_job(&spec), RouteDecision::ViewContract);
    }

    #[test]
    fn box_mm_goes_analytic() {
        let spec = JobSubmit {
            prompt: Some("box 20x10x5 mm".into()),
            ..JobSubmit::default()
        };
        assert_eq!(route_job(&spec), RouteDecision::Analytic);
    }

    #[test]
    fn neural_cad_override() {
        let spec = JobSubmit {
            prompt: Some("box 20x10x5 mm".into()),
            allow_neural_cad: true,
            ..JobSubmit::default()
        };
        assert_eq!(route_job(&spec), RouteDecision::ViewContract);
    }

    #[test]
    fn image_skips_lattice() {
        let spec = JobSubmit {
            image_path: Some("x.png".into()),
            ..JobSubmit::default()
        };
        assert_eq!(route_job(&spec), RouteDecision::Image);
    }
}
