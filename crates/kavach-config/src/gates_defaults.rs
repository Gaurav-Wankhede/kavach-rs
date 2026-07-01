// SOURCE: decision.model-shift-router-advisory
use crate::gates_config::{
    BashConfig, ContextConfig, EnforcerConfig, GatesConfig, IntentConfig, ModelRouteConfig,
    QualityConfig, ReadConfig, ResearchConfig, WriteConfig,
};
use std::collections::HashMap;

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API used cross-module"
)]
pub(crate) fn default_gates_config() -> GatesConfig {
    let mut st = HashMap::new();
    st.insert("implement".into(), vec!["rust".into(), "backend".into()]);
    st.insert("debug".into(), vec!["debug-like-expert".into()]);
    st.insert("security".into(), vec!["security".into()]);
    GatesConfig {
        schema: "kavach-gates/1.0".into(),
        description: "Default kavach gates config".into(),
        read: ReadConfig {
            enabled: true,
            blocked_paths: vec![
                "/etc/shadow".into(),
                "/etc/passwd".into(),
                "/.ssh/id_rsa".into(),
                "/.ssh/id_ed25519".into(),
                "/.aws/credentials".into(),
                "/.gnupg/".into(),
            ],
            blocked_extensions: vec![".pem".into(), ".key".into(), ".p12".into(), ".pfx".into()],
            warn_extensions: vec![".env".into(), ".secret".into()],
            warn_patterns: vec!["credentials".into(), "token".into()],
        },
        bash: BashConfig {
            enabled: true,
            blocked_commands: vec![
                // Filesystem destruction
                "rm -rf /".into(),
                "rm -rf /*".into(),
                "> /dev/sda".into(),
                ":(){ :|:& };:".into(),
                "curl | bash".into(),
                "wget | sh".into(),
                // Cloud platform destructive ops (Pocket OS incident prevention)
                "railway volume delete".into(),
                "railway project delete".into(),
                "aws ec2 terminate-instances".into(),
                "aws rds delete-db".into(),
                "aws s3 rm --recursive".into(),
                "aws cloudformation delete-stack".into(),
                "gcloud compute instances delete".into(),
                "gcloud sql instances delete".into(),
                "az group delete".into(),
                "az vm delete".into(),
                "heroku apps:destroy".into(),
                "heroku pg:reset".into(),
                "fly apps destroy".into(),
                "fly volumes destroy".into(),
                "doctl databases delete".into(),
                "doctl apps delete".into(),
                // Database destruction
                "drop database".into(),
                "truncate ".into(),
                "redis-cli flushall".into(),
                "redis-cli flushdb".into(),
                // IaC destruction
                "terraform destroy".into(),
                "pulumi destroy".into(),
                // Container destruction
                "kubectl delete namespace".into(),
                "kubectl delete --all".into(),
                // Git force push to protected branches
                "git push --force origin main".into(),
                "git push -f origin main".into(),
                "git push --force origin master".into(),
                "git push -f origin master".into(),
            ],
            blocked_patterns: vec![],
            warn_commands: vec![],
        },
        write: WriteConfig {
            enabled: true,
            blocked_paths: vec![
                "/etc/".into(),
                "/usr/".into(),
                "/bin/".into(),
                "/.ssh/".into(),
                "/.aws/".into(),
            ],
            protected_files: vec![".gitignore".into(), ".env".into(), "Cargo.lock".into()],
            secret_patterns: vec![],
        },
        enforcer: EnforcerConfig {
            enabled: true,
            chain: vec!["read".into(), "bash".into(), "write".into()],
            fail_fast: true,
        },
        intent: IntentConfig {
            enabled: true,
            skill_triggers: st,
            research_triggers: vec![],
        },
        research: ResearchConfig {
            enabled: true,
            require_before_code: true,
            code_tools: vec!["Write".into(), "Edit".into()],
            research_tools: vec!["WebSearch".into(), "WebFetch".into()],
            bypass_patterns: vec![],
        },
        context: ContextConfig {
            enabled: true,
            track_hot_paths: true,
            max_hot_files: 10,
            persist_to_stm: false,
        },
        quality: QualityConfig {
            enabled: false,
            comment: String::new(),
            check_syntax: false,
            check_imports: false,
            max_file_size_kb: 0,
        },
        updated: String::new(),
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API used cross-module"
)]
pub(crate) fn merge_gates_defaults(cfg: &mut GatesConfig) {
    let defaults = default_gates_config();
    if cfg.read.blocked_paths.is_empty() {
        cfg.read.blocked_paths = defaults.read.blocked_paths;
    }
    if cfg.bash.blocked_commands.is_empty() {
        cfg.bash.blocked_commands = defaults.bash.blocked_commands;
    }
    if cfg.write.blocked_paths.is_empty() {
        cfg.write.blocked_paths = defaults.write.blocked_paths;
    }
}
