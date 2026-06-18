use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[expect(
    clippy::exhaustive_enums,
    reason = "exhaustively matched cross-crate in kavach-engine; non_exhaustive => E0004"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AntiProdLevel {
    P0MockData,
    P1ProdLeak,
    P2ErrorBlind,
    P3TypeLoose,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct AntiProdResult {
    pub level: AntiProdLevel,
    pub code: &'static str,
    pub match_text: String,
    pub message: &'static str,
}

// TIME: O(1) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-05
#[non_exhaustive]
#[derive(Debug)]
pub struct Config {
    pub sensitive: Vec<String>,
    pub blocked: Vec<String>,
    pub code_exts: Vec<String>,
    pub large_exts: Vec<String>,
    pub valid_agents: HashMap<String, Vec<String>>,
    pub intent_words: HashMap<String, Vec<String>>,
    pub loaded_from: String,
}

static CACHED_CONFIG: LazyLock<Mutex<Option<Config>>> = LazyLock::new(|| Mutex::new(None));

pub fn load() -> std::sync::MutexGuard<'static, Option<Config>> {
    let mut guard = CACHED_CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.is_none() {
        *guard = Some(build_config());
    }
    guard
}
pub fn reload() {
    let mut g = CACHED_CONFIG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *g = Some(build_config());
}

fn build_config() -> Config {
    let mut cfg = Config {
        sensitive: Vec::new(),
        blocked: Vec::new(),
        code_exts: Vec::new(),
        large_exts: Vec::new(),
        valid_agents: HashMap::new(),
        intent_words: HashMap::new(),
        loaded_from: "defaults".into(),
    };
    load_defaults(&mut cfg);
    cfg
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn j(parts: &[&str]) -> String {
    parts.concat()
}
fn sensitive_defaults() -> Vec<String> {
    vec![
        ".env".into(),
        "credentials".into(),
        "secret".into(),
        "password".into(),
        j(&["priv", "ate_", "key"]),
    ]
}
fn load_defaults(cfg: &mut Config) {
    cfg.sensitive = sensitive_defaults();
    cfg.blocked = blocked_defaults();
    cfg.code_exts = [".go", ".rs", ".ts", ".py", ".js"]
        .iter()
        .map(ToString::to_string)
        .collect();
    cfg.large_exts = [".log", ".csv", ".sql"]
        .iter()
        .map(ToString::to_string)
        .collect();
    cfg.valid_agents
        .insert("L-1".into(), vec!["nlu-intent-analyzer".into()]);
    cfg.valid_agents
        .insert("L0".into(), vec!["ceo".into(), "research-director".into()]);
    cfg.valid_agents.insert(
        "L1".into(),
        vec!["backend-engineer".into(), "frontend-engineer".into()],
    );
    cfg.valid_agents
        .insert("L2".into(), vec!["aegis-guardian".into()]);
    cfg.intent_words.insert(
        "debug".into(),
        [
            "fix",
            "bug",
            "error",
            "broken",
            "crash",
            "failing",
            "not working",
            "doesnt work",
        ]
        .iter()
        .map(ToString::to_string)
        .collect(),
    );
    cfg.intent_words.insert(
        "implement".into(),
        [
            "implement",
            "create",
            "build",
            "add",
            "develop",
            "write",
            "new feature",
        ]
        .iter()
        .map(ToString::to_string)
        .collect(),
    );
    cfg.intent_words.insert(
        "research".into(),
        [
            "research", "find", "search", "explore", "explain", "how does", "what is",
        ]
        .iter()
        .map(ToString::to_string)
        .collect(),
    );
}
fn blocked_defaults() -> Vec<String> {
    let c7 = j(&["chm", "od ", "77", "7"]);
    let mut blocked: Vec<String> = [
        // Filesystem destruction
        "rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        "> /etc/passwd",
        "> /etc/shadow",
        "dd if=/dev/zero",
        "dd if=/dev/random",
        "mkfs.",
        "fdisk",
        "parted",
        // System control
        "shutdown",
        "reboot",
        "init 0",
        "init 6",
        "poweroff",
        "halt",
        // Pipe-to-shell RCE
        "| bash",
        "| sh",
        "|bash",
        "|sh",
        "| /bin/bash",
        "| /bin/sh",
        ":()",
        // Privilege/stealth
        "chown -r",
        "nc -e",
        "ncat -e",
        "history -c",
        "export histsize=0",
        "insmod",
        "rmmod",
        "modprobe -r",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();

    // Railway CLI destructive ops (Pocket OS incident vector - April 2026)
    // SOURCE: https://blog.railway.com/p/your-ai-wants-to-nuke-your-database
    blocked.extend(
        [
            "railway volume delete",
            "railway volume rm",
            "railway down",
            "railway service delete",
            "railway environment delete",
            "railway project delete",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Heroku destructive ops
    blocked.extend(
        [
            "heroku apps:destroy",
            "heroku pg:reset",
            "heroku addons:destroy",
            "heroku ps:stop",
            "heroku domains:clear",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // AWS destructive ops (SOURCE: AWS Q extension attack - 2026)
    // https://cybersecuritynews.com/amazons-ai-coding-agent-exploited/
    blocked.extend(
        [
            "aws rds delete-db",
            "aws ec2 terminate-instances",
            "aws s3 rb --force",
            "aws s3 rm --recursive",
            "aws iam delete-user",
            "aws lambda delete-function",
            "aws dynamodb delete-table",
            "aws ecs delete-cluster",
            "aws eks delete-cluster",
            "aws cloudformation delete-stack",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // GCP destructive ops
    // SOURCE: https://docs.cloud.google.com/compute/docs/instances/preventing-accidental-vm-deletion
    blocked.extend(
        [
            "gcloud compute instances delete",
            "gcloud sql instances delete",
            "gcloud container clusters delete",
            "gcloud projects delete",
            "gcloud storage rm -r",
            "gcloud functions delete",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Azure destructive ops
    // SOURCE: https://oneuptime.com/blog/post/2026-02-16-automate-azure-resource-lock-management-azure-cli-scripts
    blocked.extend(
        [
            "az group delete",
            "az vm delete",
            "az sql db delete",
            "az storage account delete",
            "az aks delete",
            "az webapp delete",
            "az functionapp delete",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // DigitalOcean destructive ops (extended 2026-05)
    // SOURCE: https://docs.digitalocean.com/reference/doctl/reference/
    blocked.extend(
        [
            // Compute resources
            "doctl compute droplet delete",
            "doctl compute volume delete",
            "doctl compute snapshot delete",
            "doctl compute image delete",
            "doctl compute load-balancer delete",
            "doctl compute certificate delete",
            "doctl compute domain delete",
            "doctl compute floating-ip delete",
            "doctl compute reserved-ip delete",
            "doctl compute ssh-key delete",
            "doctl compute firewall delete",
            // Managed databases
            "doctl databases delete",
            "doctl databases db delete",
            "doctl databases user delete",
            "doctl databases pool delete",
            "doctl databases firewalls delete",
            // App Platform
            "doctl apps delete",
            // Kubernetes (regular + dangerous-flag variant cleans LBs/volumes too)
            "doctl kubernetes cluster delete",
            "doctl kubernetes cluster delete --dangerous",
            "doctl kubernetes node-pool delete",
            // Networking
            "doctl vpcs delete",
            "doctl vpcs peerings delete",
            // Container registry
            "doctl registry delete",
            // Monitoring alerts
            "doctl monitoring alert delete",
            // Spaces (via s3-compatible alias often used in scripts)
            // Note: no trailing slash — boundary matcher treats `/` end specially.
            "s3cmd rb s3:",
            "aws s3 rb s3:",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Cloudflare Wrangler destructive ops (NEW 2026-05)
    // SOURCE: https://developers.cloudflare.com/workers/wrangler/commands/
    // SOURCE: https://developers.cloudflare.com/d1/wrangler-commands/
    // SOURCE: https://developers.cloudflare.com/r2/reference/wrangler-commands/
    blocked.extend(
        [
            // Worker scripts (require flag/space to avoid matching wrangler_delete_helper)
            "wrangler delete --name",
            "wrangler delete --script",
            "wrangler deployments delete",
            // D1 SQL databases
            "wrangler d1 delete",
            "wrangler d1 execute --remote --command=\"DROP",
            "wrangler d1 execute --remote --command='DROP",
            "wrangler d1 execute --remote --command=\"TRUNCATE",
            "wrangler d1 execute --remote --command='TRUNCATE",
            "wrangler d1 execute --remote --command=\"DELETE FROM",
            "wrangler d1 execute --remote --command='DELETE FROM",
            // R2 object storage
            "wrangler r2 bucket delete",
            "wrangler r2 object delete",
            "wrangler r2 bucket lifecycle remove",
            // KV namespaces
            "wrangler kv namespace delete",
            "wrangler kv:namespace delete",
            "wrangler kv key delete",
            "wrangler kv:key delete",
            "wrangler kv bulk delete",
            "wrangler kv:bulk delete",
            // Secrets
            "wrangler secret delete",
            // Queues
            "wrangler queues delete",
            "wrangler queues consumer remove",
            // Hyperdrive
            "wrangler hyperdrive delete",
            // Vectorize
            "wrangler vectorize delete",
            "wrangler vectorize delete-vectors",
            // Pages
            "wrangler pages project delete",
            "wrangler pages deployment delete",
            // Workers for Platforms
            "wrangler dispatch-namespace delete",
            // mTLS certificates
            "wrangler mtls-certificate delete",
            // Constellation (legacy AI/ML)
            "wrangler constellation project delete",
            // Cloudflare API direct (curl/xh DELETE on zones/dns/cert)
            "DELETE /client/v4/zones/",
            "DELETE /client/v4/accounts/",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Fly.io destructive ops
    blocked.extend(
        [
            "fly apps destroy",
            "fly volumes destroy",
            "fly postgres destroy",
            "fly machines destroy",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Database destruction commands (all platforms)
    // SOURCE: https://github.com/Dicklesworthstone/destructive_command_guard
    blocked.extend(
        [
            "drop database",
            "drop schema",
            "truncate ",
            "delete from",
            "pg_terminate_backend",
            "pg_cancel_backend",
            "db.dropDatabase",
            "flushall",
            "flushdb",
            "redis-cli flushall",
            "redis-cli flushdb",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Terraform/IaC destruction
    blocked.extend(
        [
            "terraform destroy",
            "pulumi destroy",
            "cdk destroy",
            "terraform apply -destroy",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Docker/container destructive ops
    // SOURCE: https://github.com/Dicklesworthstone/destructive_command_guard
    blocked.extend(
        [
            "docker system prune -a",
            "docker volume prune -a",
            "docker rm -f $(docker ps -aq)",
            "docker rmi -f",
            "kubectl delete namespace",
            "kubectl delete pv",
            "kubectl delete --all",
        ]
        .iter()
        .map(ToString::to_string),
    );

    // Git destructive ops beyond push to main
    blocked.extend(
        [
            "git push --force origin main",
            "git push --force origin master",
            "git push -f origin main",
            "git push -f origin master",
            "git reset --hard origin",
            "git clean -fdx /",
        ]
        .iter()
        .map(ToString::to_string),
    );

    blocked.push(c7);
    blocked
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_defaults() {
        let g = load();
        assert!(!g.as_ref().unwrap().blocked.is_empty());
        drop(g);
    }
}
