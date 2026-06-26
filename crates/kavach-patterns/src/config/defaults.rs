use super::j;
use super::types::Config;

pub(super) fn sensitive_defaults() -> Vec<String> {
    vec![
        ".env".into(),
        "credentials".into(),
        "secret".into(),
        "password".into(),
        j(&["priv", "ate_", "key"]),
    ]
}

pub(super) fn load_defaults(cfg: &mut Config) {
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

pub(super) fn blocked_defaults() -> Vec<String> {
    let c7 = j(&["chm", "od ", "77", "7"]);
    let mut blocked: Vec<String> = [
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
        "shutdown",
        "reboot",
        "init 0",
        "init 6",
        "poweroff",
        "halt",
        "| bash",
        "| sh",
        "|bash",
        "|sh",
        "| /bin/bash",
        "| /bin/sh",
        ":(",
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

    blocked.extend(
        [
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
            "doctl databases delete",
            "doctl databases db delete",
            "doctl databases user delete",
            "doctl databases pool delete",
            "doctl databases firewalls delete",
            "doctl apps delete",
            "doctl kubernetes cluster delete",
            "doctl kubernetes cluster delete --dangerous",
            "doctl kubernetes node-pool delete",
            "doctl vpcs delete",
            "doctl vpcs peerings delete",
            "doctl registry delete",
            "doctl monitoring alert delete",
            "s3cmd rb s3:",
            "aws s3 rb s3:",
        ]
        .iter()
        .map(ToString::to_string),
    );

    blocked.extend(
        [
            "wrangler delete --name",
            "wrangler delete --script",
            "wrangler deployments delete",
            "wrangler d1 delete",
            "wrangler d1 execute --remote --command=\"DROP",
            "wrangler d1 execute --remote --command='DROP",
            "wrangler d1 execute --remote --command=\"TRUNCATE",
            "wrangler d1 execute --remote --command='TRUNCATE",
            "wrangler d1 execute --remote --command=\"DELETE FROM",
            "wrangler d1 execute --remote --command='DELETE FROM",
            "wrangler r2 bucket delete",
            "wrangler r2 object delete",
            "wrangler r2 bucket lifecycle remove",
            "wrangler kv namespace delete",
            "wrangler kv:namespace delete",
            "wrangler kv key delete",
            "wrangler kv:key delete",
            "wrangler kv bulk delete",
            "wrangler kv:bulk delete",
            "wrangler secret delete",
            "wrangler queues delete",
            "wrangler queues consumer remove",
            "wrangler hyperdrive delete",
            "wrangler vectorize delete",
            "wrangler vectorize delete-vectors",
            "wrangler pages project delete",
            "wrangler pages deployment delete",
            "wrangler dispatch-namespace delete",
            "wrangler mtls-certificate delete",
            "wrangler constellation project delete",
            "DELETE /client/v4/zones/",
            "DELETE /client/v4/accounts/",
        ]
        .iter()
        .map(ToString::to_string),
    );

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
