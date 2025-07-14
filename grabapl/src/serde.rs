use crate::Semantics;
use crate::operation::user_defined::UserDefinedOperation;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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
