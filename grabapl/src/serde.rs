use crate::Semantics;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(crate) trait SemanticsSerde:
    Semantics<
        BuiltinQuery: Serialize + DeserializeOwned,
        BuiltinOperation: Serialize + DeserializeOwned,
        NodeAbstract: Serialize + DeserializeOwned,
        NodeConcrete: Serialize + DeserializeOwned,
        // note: 'static is needed since serde_json_any_key::any_key_map has the wrong bounds.
        // PR or fork could remove the 'static requirement.
        EdgeAbstract: Serialize + DeserializeOwned + 'static,
        EdgeConcrete: Serialize + DeserializeOwned,
    >
{
}

impl<
    S: Semantics<
            BuiltinQuery: Serialize + DeserializeOwned,
            BuiltinOperation: Serialize + DeserializeOwned,
            NodeAbstract: Serialize + DeserializeOwned,
            NodeConcrete: Serialize + DeserializeOwned,
            EdgeAbstract: Serialize + DeserializeOwned + 'static,
            EdgeConcrete: Serialize + DeserializeOwned,
        >,
> SemanticsSerde for S
{
}
