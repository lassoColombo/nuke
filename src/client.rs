use anyhow::Result;
use kube::Config;

pub fn kubeconfig_path() -> std::path::PathBuf {
    std::env::var("KUBECONFIG")
        .ok()
        .and_then(|s| s.split(':').next().map(std::path::PathBuf::from))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .expect("no home dir")
                .join(".kube")
                .join("config")
        })
}

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
