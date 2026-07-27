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
        model: ModelRouteConfig::default(),
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
                "rm -rf ~".into(),
                "> /dev/sda".into(),
                "> /etc/passwd".into(),
                "> /etc/shadow".into(),
                ":(){ :|:& };:".into(),
                // Disk/device destruction
                "dd if=/dev/zero".into(),
                "dd if=/dev/random".into(),
                "mkfs.".into(),
                "fdisk".into(),
                "parted".into(),
                // Power/system ops
                "shutdown".into(),
                "reboot".into(),
                "init 0".into(),
                "init 6".into(),
                "poweroff".into(),
                "halt".into(),
                // Pipe-to-shell
                "curl | bash".into(),
                "curl | sh".into(),
                "wget | sh".into(),
                "wget | bash".into(),
                "| bash".into(),
                "| sh".into(),
                "|bash".into(),
                "|sh".into(),
                "| /bin/bash".into(),
                "| /bin/sh".into(),
                // Reverse shells
                "nc -e".into(),
                "ncat -e".into(),
                // History cleanup
                "history -c".into(),
                "export histsize=0".into(),
                // Kernel modules
                "insmod".into(),
                "rmmod".into(),
                "modprobe -r".into(),
                // Recursive chown on system dirs
                "chown -r".into(),
                // Cloud platform destructive ops (Pocket OS incident prevention)
                "railway volume delete".into(),
                "railway volume rm".into(),
                "railway down".into(),
                "railway service delete".into(),
                "railway environment delete".into(),
                "railway project delete".into(),
                "aws ec2 terminate-instances".into(),
                "aws rds delete-db".into(),
                "aws s3 rm --recursive".into(),
                "aws s3 rb --force".into(),
                "aws iam delete-user".into(),
                "aws lambda delete-function".into(),
                "aws dynamodb delete-table".into(),
                "aws ecs delete-cluster".into(),
                "aws eks delete-cluster".into(),
                "aws cloudformation delete-stack".into(),
                "gcloud compute instances delete".into(),
                "gcloud sql instances delete".into(),
                "gcloud container clusters delete".into(),
                "gcloud projects delete".into(),
                "gcloud storage rm -r".into(),
                "gcloud functions delete".into(),
                "az group delete".into(),
                "az vm delete".into(),
                "az sql db delete".into(),
                "az storage account delete".into(),
                "az aks delete".into(),
                "az webapp delete".into(),
                "az functionapp delete".into(),
                "heroku apps:destroy".into(),
                "heroku pg:reset".into(),
                "heroku addons:destroy".into(),
                "heroku ps:stop".into(),
                "heroku domains:clear".into(),
                "fly apps destroy".into(),
                "fly volumes destroy".into(),
                "fly postgres destroy".into(),
                "fly machines destroy".into(),
                "doctl compute droplet delete".into(),
                "doctl compute volume delete".into(),
                "doctl compute snapshot delete".into(),
                "doctl compute image delete".into(),
                "doctl compute load-balancer delete".into(),
                "doctl compute certificate delete".into(),
                "doctl compute domain delete".into(),
                "doctl compute floating-ip delete".into(),
                "doctl compute reserved-ip delete".into(),
                "doctl compute ssh-key delete".into(),
                "doctl compute firewall delete".into(),
                "doctl databases delete".into(),
                "doctl databases db delete".into(),
                "doctl databases user delete".into(),
                "doctl databases pool delete".into(),
                "doctl databases firewalls delete".into(),
                "doctl apps delete".into(),
                "doctl kubernetes cluster delete".into(),
                "doctl kubernetes cluster delete --dangerous".into(),
                "doctl kubernetes node-pool delete".into(),
                "doctl vpcs delete".into(),
                "doctl vpcs peerings delete".into(),
                "doctl registry delete".into(),
                "doctl monitoring alert delete".into(),
                "s3cmd rb s3:".into(),
                "aws s3 rb s3:".into(),
                // Wrangler/Cloudflare destructive ops
                "wrangler delete --name".into(),
                "wrangler delete --script".into(),
                "wrangler deployments delete".into(),
                "wrangler d1 delete".into(),
                "wrangler d1 execute --remote --command=\"DROP".into(),
                "wrangler d1 execute --remote --command='DROP".into(),
                "wrangler d1 execute --remote --command=\"TRUNCATE".into(),
                "wrangler d1 execute --remote --command='TRUNCATE".into(),
                "wrangler d1 execute --remote --command=\"DELETE FROM".into(),
                "wrangler d1 execute --remote --command='DELETE FROM".into(),
                "wrangler r2 bucket delete".into(),
                "wrangler r2 object delete".into(),
                "wrangler r2 bucket lifecycle remove".into(),
                "wrangler kv namespace delete".into(),
                "wrangler kv:namespace delete".into(),
                "wrangler kv key delete".into(),
                "wrangler kv:key delete".into(),
                "wrangler kv bulk delete".into(),
                "wrangler kv:bulk delete".into(),
                "wrangler secret delete".into(),
                "wrangler queues delete".into(),
                "wrangler queues consumer remove".into(),
                "wrangler hyperdrive delete".into(),
                "wrangler vectorize delete".into(),
                "wrangler vectorize delete-vectors".into(),
                "wrangler pages project delete".into(),
                "wrangler pages deployment delete".into(),
                "wrangler dispatch-namespace delete".into(),
                "wrangler mtls-certificate delete".into(),
                "wrangler constellation project delete".into(),
                "DELETE /client/v4/zones/".into(),
                "DELETE /client/v4/accounts/".into(),
                // Database destruction
                "drop database".into(),
                "drop schema".into(),
                "truncate ".into(),
                "delete from".into(),
                "pg_terminate_backend".into(),
                "pg_cancel_backend".into(),
                "db.dropDatabase".into(),
                "flushall".into(),
                "flushdb".into(),
                "redis-cli flushall".into(),
                "redis-cli flushdb".into(),
                // IaC destruction
                "terraform destroy".into(),
                "pulumi destroy".into(),
                "cdk destroy".into(),
                "terraform apply -destroy".into(),
                // Container destruction
                "docker system prune -a".into(),
                "docker volume prune -a".into(),
                "docker rm -f $(docker ps -aq)".into(),
                "docker rmi -f".into(),
                "kubectl delete namespace".into(),
                "kubectl delete pv".into(),
                "kubectl delete --all".into(),
                // Git force push to protected branches
                "git push --force origin main".into(),
                "git push --force origin master".into(),
                "git push -f origin main".into(),
                "git push -f origin master".into(),
                "git reset --hard origin".into(),
                "git clean -fdx /".into(),
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
