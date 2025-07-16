/*!
Not implemented :(
My idea to implement this is as follows:

The user can specify a set of concrete examples.
A single such example consists of a concrete graph, that must match the operation's signature, but
could have more nodes as well (to account for shape queries!)

When stepping through the abstract operation, the example is ran through each instruction as well.
However, since some branches might not be entered for the example, that case needs to be handled.
For example, you might store an Option<ExecutedExample> and set it to none when entering a branch that
concretely is not entered for that example.

We might even run the entire set of examples simultaneously, keep track of which dynamically reach the current
instruction, and whenever that set is empty, we can tell the user somehow and they can then add more
examples to the operation.

*/
