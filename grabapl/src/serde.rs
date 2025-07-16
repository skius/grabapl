use crate::Semantics;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub(crate) trait SemanticsSerde:
    Semantics<
        BuiltinQuery: Serialize + DeserializeOwned,
        BuiltinOperation: Serialize + DeserializeOwned,
        NodeAbstract: Serialize + DeserializeOwned,
        NodeConcrete: Serialize + DeserializeOwned,
        EdgeAbstract: Serialize + DeserializeOwned,
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
            EdgeAbstract: Serialize + DeserializeOwned,
            EdgeConcrete: Serialize + DeserializeOwned,
        >,
> SemanticsSerde for S
{
}
