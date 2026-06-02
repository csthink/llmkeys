//! 目录三层合并(spec S3)。
//!
//! 优先级(低 → 高):内置快照 < models.dev 缓存 < 用户 overrides。
//! **字段级合并**:高层非空字段覆盖低层,低层其余字段保留;**用户写的永远赢**。
//!
//! `models.dev` 层只用于 **enrich 已知 provider**(快照∪overrides 出现过的 id),不把上游
//! 138 家裸条目灌进 `list`(保持列表精炼),且其字段贡献限于 `display_name`(见 [`modelsdev`])。

pub mod modelsdev;
pub mod overrides;
pub mod snapshot;

use std::collections::BTreeSet;

use anyhow::Result;

use crate::model::{Provider, ProvidersFile};

/// 把 `hi` 层的非空字段合并进 `base`(字段级,高层赢)。
fn merge_into(base: &mut Provider, hi: Provider) {
    if hi.display_name.is_some() {
        base.display_name = hi.display_name;
    }
    if hi.base_url.is_some() {
        base.base_url = hi.base_url;
    }
    if hi.key_ref.is_some() {
        base.key_ref = hi.key_ref;
    }
    if hi.env_prefix.is_some() {
        base.env_prefix = hi.env_prefix;
    }
    if hi.naming_note.is_some() {
        base.naming_note = hi.naming_note;
    }
    // 模型角色逐角色合并:高层同名角色覆盖,低层其余角色保留。
    for (role, id) in hi.models.roles {
        base.models.roles.insert(role, id);
    }
}

/// 按从低到高的顺序字段级合并多层 provider 配置。
pub fn merge(layers: impl IntoIterator<Item = ProvidersFile>) -> ProvidersFile {
    let mut out = ProvidersFile::default();
    for layer in layers {
        for (id, p) in layer.providers {
            merge_into(out.providers.entry(id).or_default(), p);
        }
    }
    out
}

/// 读取三层并合并出最终目录:快照 < models.dev(enrich 已知) < overrides。
///
/// models.dev 缓存缺失/损坏不致失败(退回快照,S6);overrides 缺文件按空层处理。
pub fn load_merged() -> Result<ProvidersFile> {
    let snapshot = snapshot::load()?;
    let overrides = overrides::load()?;

    // 只让 models.dev enrich 已知 provider(快照∪overrides),不引入上游裸条目。
    let known: BTreeSet<String> = snapshot
        .providers
        .keys()
        .chain(overrides.providers.keys())
        .cloned()
        .collect();
    let mut modelsdev = modelsdev::load_layer();
    modelsdev.providers.retain(|id, _| known.contains(id));

    Ok(merge([snapshot, modelsdev, overrides]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Models;

    /// 便捷构造一个 provider(只设关心的字段)。
    fn prov(fields: Provider) -> Provider {
        fields
    }

    fn file(pairs: Vec<(&str, Provider)>) -> ProvidersFile {
        let mut f = ProvidersFile::default();
        for (id, p) in pairs {
            f.providers.insert(id.to_string(), p);
        }
        f
    }

    fn models(pairs: &[(&str, &str)]) -> Models {
        let mut m = Models::default();
        for (k, v) in pairs {
            m.roles.insert(k.to_string(), v.to_string());
        }
        m
    }

    #[test]
    fn field_level_precedence_across_three_layers() {
        let snapshot = file(vec![(
            "x",
            prov(Provider {
                display_name: Some("snap".into()),
                base_url: Some("https://snap/v1".into()),
                key_ref: Some("keychain:x".into()),
                models: models(&[("chat", "c-snap")]),
                ..Default::default()
            }),
        )]);
        let modelsdev = file(vec![(
            "x",
            prov(Provider {
                // models.dev 只带 display_name。
                display_name: Some("from-models-dev".into()),
                ..Default::default()
            }),
        )]);
        let overrides = file(vec![(
            "x",
            prov(Provider {
                base_url: Some("https://user/v1".into()),
                models: models(&[("embedding", "e-user")]),
                ..Default::default()
            }),
        )]);

        let merged = merge([snapshot, modelsdev, overrides]);
        let x = &merged.providers["x"];
        // overrides 覆盖 base_url(用户赢)。
        assert_eq!(x.base_url.as_deref(), Some("https://user/v1"));
        // display_name 来自 models.dev(覆盖快照)。
        assert_eq!(x.display_name.as_deref(), Some("from-models-dev"));
        // 没人覆盖的字段保留快照值。
        assert_eq!(x.key_ref.as_deref(), Some("keychain:x"));
        // 模型角色逐角色合并:chat 来自快照,embedding 来自 overrides。
        assert_eq!(x.models.chat(), Some("c-snap"));
        assert_eq!(x.models.embedding(), Some("e-user"));
    }

    #[test]
    fn override_replaces_single_field_only() {
        // DoD:overrides 能覆盖快照单个字段,其余字段不动。
        let snapshot = file(vec![(
            "openrouter",
            prov(Provider {
                display_name: Some("OpenRouter".into()),
                base_url: Some("https://openrouter.ai/api/v1".into()),
                key_ref: Some("keychain:openrouter".into()),
                env_prefix: Some("OPENROUTER".into()),
                ..Default::default()
            }),
        )]);
        let overrides = file(vec![(
            "openrouter",
            prov(Provider {
                base_url: Some("https://my-proxy.local/v1".into()),
                ..Default::default()
            }),
        )]);

        let merged = merge([snapshot, ProvidersFile::default(), overrides]);
        let p = &merged.providers["openrouter"];
        assert_eq!(p.base_url.as_deref(), Some("https://my-proxy.local/v1"));
        // 其余字段仍是快照的。
        assert_eq!(p.key_ref.as_deref(), Some("keychain:openrouter"));
        assert_eq!(p.env_prefix.as_deref(), Some("OPENROUTER"));
        assert_eq!(p.display_name.as_deref(), Some("OpenRouter"));
    }

    #[test]
    fn snapshot_only_yields_full_offline_catalog() {
        // DoD:无网络(models.dev 空、overrides 空)时 list 仍可用 —— 全走快照。
        let snapshot = snapshot::load().unwrap();
        let merged = merge([
            snapshot.clone(),
            ProvidersFile::default(),
            ProvidersFile::default(),
        ]);
        assert_eq!(merged.providers.len(), snapshot.providers.len());
        assert_eq!(
            merged.providers["deepseek"].base_url.as_deref(),
            Some("https://api.deepseek.com/v1")
        );
    }

    #[test]
    fn empty_layer_does_not_introduce_or_erase() {
        let snapshot = file(vec![(
            "x",
            prov(Provider {
                base_url: Some("https://snap/v1".into()),
                ..Default::default()
            }),
        )]);
        let merged = merge([snapshot, ProvidersFile::default()]);
        assert_eq!(merged.providers.len(), 1);
        assert_eq!(merged.providers["x"].base_url.as_deref(), Some("https://snap/v1"));
    }
}
