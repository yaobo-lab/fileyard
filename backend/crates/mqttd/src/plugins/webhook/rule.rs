use bytestring::ByteString;
use rmqtt::{trie::TopicTree, types::Topic};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Unexpected},
    ser,
};
use std::str::FromStr;
use std::sync::Arc;

pub(super) type TopicsType = Option<(Arc<TopicTree<()>>, Vec<String>)>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct Rule {
    pub action: String,
    #[serde(default)]
    pub urls: Vec<Url>,
    #[serde(
        default,
        deserialize_with = "Rule::deserialize_topics",
        serialize_with = "Rule::serialize_topics"
    )]
    pub topics: TopicsType,
}

impl Rule {
    pub(super) fn serialize_topics<S>(
        topics: &TopicsType,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        if let Some((_, topics_cfg)) = topics {
            topics_cfg.as_slice().serialize(s)
        } else {
            let topics_cfg: Vec<String> = Vec::new();
            topics_cfg.as_slice().serialize(s)
        }
    }

    pub(super) fn deserialize_topics<'de, D>(
        deserializer: D,
    ) -> std::result::Result<TopicsType, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let topics_cfg: Vec<String> = Vec::deserialize(deserializer)?;

        if topics_cfg.is_empty() {
            Ok(None)
        } else {
            let mut topics = TopicTree::default();
            for topic in topics_cfg.iter() {
                topics.insert(
                    &Topic::from_str(topic).map_err(|e| de::Error::custom(format!("{e:?}")))?,
                    (),
                );
            }
            Ok(Some((Arc::new(topics), topics_cfg)))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Url {
    pub loc: ByteString,
    pub typ: UrlType,
}

impl Url {
    #[inline]
    pub(super) fn is_file(&self) -> bool {
        matches!(self.typ, UrlType::File)
    }
}

impl<'de> Deserialize<'de> for Url {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let loc = String::deserialize(deserializer)?;
        let loc = loc.trim();
        let uri = url::Url::parse(loc).map_err(de::Error::custom)?;
        let (typ, loc) = if uri.scheme() == "http" || uri.scheme() == "https" {
            (UrlType::Http, loc)
        } else if uri.scheme() == "file" {
            (UrlType::File, uri.path())
        } else {
            return Err(de::Error::invalid_value(
                Unexpected::Str(loc),
                &"http:// or https:// or file://",
            ));
        };
        Ok(Url {
            loc: ByteString::from(loc),
            typ,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum UrlType {
    File,
    Http,
}
