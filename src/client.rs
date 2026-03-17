use anyhow::Result;
use kube::Config;

pub async fn config_from_context(context: Option<String>) -> Result<Config> {
    if let Some(ctx) = context {
        let options = kube::config::KubeConfigOptions {
            context: Some(ctx),
            cluster: None,
            user: None,
        };
        Ok(Config::from_kubeconfig(&options).await?)
    } else {
        Ok(Config::infer().await?)
    }
}
