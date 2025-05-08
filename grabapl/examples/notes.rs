/*
TODO s from old `match to pattern:

TODO: Implement priority based on closeness of siblings. If the pattern expects two siblings, then we should prefer in A->{B,C,D} the subgraph A->{B,C} or A->{C,D} over A->{B,D}.
    We should however also support A->{D,A} as mapping for example, since we want circular orders.
TODO: I propose doing this via a hard and soft check of orders:
    * The hard check checks that there is no going back and forth for >2 siblings, or, in other words, for some picked starting point of the circular order, the remaining children are in-order of at most a full loop.
    * The soft check prioritizes the returned results such that the first child is preferably also the first child, and any siblings are as close as possible to the input node. If we want to expand this definition, we could say we proceed in BFS order.
TODO: Add circular order to the child order


TODO: Add option to ignore parent order?


*/

fn main() {}
