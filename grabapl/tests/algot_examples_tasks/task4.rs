//! # Task 4: Smallest Distance Between Consecutive Integers
//! The function f should return the smallest distance of two consecutive integers (in ascending order) in the list.
//! For example, [1, 2, 3] would return 1, and [3, 2, 1, 4] would return 3.
//! f uses an auxiliary function aux.

use grabapl::prelude::*;
use syntax::grabapl_defs;
use crate::util::semantics::*;

grabapl_defs!(get_ops, TestSemantics,
    fn min_dist_of_consecutive_integers(list: Integer) -> (min_dist: Integer) {
        let! min_dist = add_node<int, 0>();

        return (min_dist: min_dist);
    }




);

#[test_log::test]
fn task4() {

}