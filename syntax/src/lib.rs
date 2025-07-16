/*
This crate should eventually support parsing functions such as:

def max_heap_remove(sentinel: Object) -> (max_value: Integer) {
    // ! syntax: take the single return value of the operation and bind to it.
    // alternative is let map = add_node(...); and then map.new is the returned node.
    let! max_value = add_node(-1);
    if shape [
        root: Integer,
        sentinel -> root: Wildcard
    ] {
        // if we have a root, we can proceed
        max_heap_remove_helper(root, max_value);
    } else {
        // do nothing
    }
    return (max_value: max_value);
}


def max_heap_remove_helper(root: Integer, max_value: Integer) {
    // return the value of the root node
    copy_value_from_to(root, max_value);
    if shape [
        left: Integer,
        root -> left: Wildcard,
        right: Integer,
        root -> right: Wildcard
    ] {
        // we have two children, check which is larger
        // method[] syntax: pass arguments to builtin operations
        if cmp_fst_snd[>](left, right) {
            // left is larger, recurse on left
            let! temp_max = add_node(-1);
            max_heap_remove_helper(left, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        } else {
            // right is larger or equal, recurse on right
            let! temp_max = add_node(-1);
            max_heap_remove_helper(right, temp_max);
            copy_value_from_to(temp_max, root);
            remove_node(temp_max);
        }
    } else if shape [
        child: Integer,
        root -> child: Wildcard
    ] {
        // we have one child, recurse on it
        let! temp_max = add_node(int(-1));
        max_heap_remove_helper(child, temp_max);
        copy_value_from_to(temp_max, root);
        remove_node(temp_max);
    } else {
        // no children, we can delete the root node
        remove_node(root);
    }
}


// some more syntax:

// ?ident marks a node as a context node that will implicitly be matched.
def takes_context_nodes(child: Integer, ?parent: Object, child -> parent: "child") {

}

*/