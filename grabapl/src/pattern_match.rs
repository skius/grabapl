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
    // TODO: define variants with and without wraparound?
    ExactOffset(NonZeroI32),
    // Probably *dont* want these to be wraparound, since otherwise they're useless.
    // a wraparound "before" constraint on two edges is meaningless.
    // It would only become meaningful if you had more than two edges and defined a total order
    // over all three with the constraint that it must be <= one full loop.
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

how are edges ordered for a given visual representation? I would say according to their visual position.

Can the user override a given edge? eg by also passing the node it should match to? seems complicated.



What if we wanted to represent current Algot Web semantics using this system?
Going 'outwards' from the input node, every edge gets an Absolute(FromStart(..)) order. Once we pivot, we get a "Relative(ExactOffset(+/-1))" order.


TODO Big question:
How to handle incoming vs. outgoing edges? Independently? i.e., for dir in [Incoming, Outgoing], we have a separate
order incl separate LocalEdgeId etc?
Think about if that can cause issues!




*/

/* TODO: find examples of why the old "ordered but wraparound" child approach is not good

Maybe because we want to potentially stop at the last edge? See BFS in Algot Web. If we didn't stop, we'd infinitely loop.
But also, we do want to potentially wrap around. So we cannot just _not_ wraparound. An example for this is maybe a generic BFS?

 */
