use std::num::NonZeroI32;

type LocalEdgeId = u32;

struct Edge {
    source: EdgeEndpointOrderInfo,
    target: EdgeEndpointOrderInfo,
}

struct EdgeEndpointOrderInfo {
    id: LocalEdgeId,
    order: MatchOrder,
}

enum MatchOrder {
    Any,
    Absolute(AbsoluteMatchOrder),
    Relative {
        anchor: LocalEdgeId,
        relative_order: RelativeMatchOrder,
    },
}

enum AbsoluteMatchOrder {
    FromStart(usize),
    FromEnd(usize),
}

enum RelativeMatchOrder {
    ExactOffset(NonZeroI32), // TODO: define variants with and without wraparound?
    Before,
    After,
}

/*
Ok, but now what happens if we have a nested operation call?

Say:
Concrete graph: X->{Y, Z}

Outer operation: input A with "FromEnd(0)" edge to B
i.e., A=>X, B=>Z is the substitution that will be used

Inner operation: input A' with "FromStart(0)" edge to B'
Should we now use the abstract graph from outer operation? In which case, FromStart(0) will match A->B?
Or should we go back all the way to the concrete graph, in which case we would get the X->Y edge?

I probably think the former, even though that may be confusing? 

*/

/* TODO: find examples of why the old "ordered but wraparound" child approach is not good

Maybe because we want to potentially stop at the last edge? See BFS in Algot Web. If we didn't stop, we'd infinitely loop.
But also, we do want to potentially wrap around. So we cannot just _not_ wraparound. An example for this is maybe a generic BFS?

 */
