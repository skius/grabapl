use std::{fmt, mem};
use std::sync::Arc;
use proptest::prelude::Strategy;
use proptest::strategy::{Fuse, Map, NewTree, ValueTree};
use proptest::test_runner::{Reason, TestRunner};

pub trait StrategyOutsideFirstExtension: Strategy {
    fn proptest_flat_map_outside_first<S: Strategy, F: Fn(Self::Value) -> S>(
        self,
        fun: F,
    ) -> FlattenOutsideFirst<Map<Self, F>>
    where
        Self: Sized,
    {
        FlattenOutsideFirst::new(self.prop_map(fun))
    }
}

impl<S: Strategy> StrategyOutsideFirstExtension for S {}

/// Adaptor that flattens a `Strategy` which produces other `Strategy`s into a
/// `Strategy` that picks one of those strategies and then picks values from
/// it.
#[derive(Debug, Clone, Copy)]
#[must_use = "strategies do nothing unless used"]
pub struct FlattenOutsideFirst<S> {
    source: S,
}

impl<S: Strategy> FlattenOutsideFirst<S> {
    /// Wrap `source` to flatten it.
    pub fn new(source: S) -> Self {
        FlattenOutsideFirst { source }
    }
}

impl<S: Strategy> Strategy for FlattenOutsideFirst<S>
where
    S::Value: Strategy,
    <S::Value as Strategy>::Tree: Clone,
{
    type Tree = FlattenOutsideFirstValueTree<S::Tree>;
    type Value = <S::Value as Strategy>::Value;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        let meta = self.source.new_tree(runner)?;
        FlattenOutsideFirstValueTree::new(runner, meta)
    }
}

/// The `ValueTree` produced by `Flatten`.
pub struct FlattenOutsideFirstValueTree<S: ValueTree>
where
    S::Value: Strategy,
{
    meta: Fuse<S>,
    current: Fuse<<S::Value as Strategy>::Tree>,
    last_complication: Option<Fuse<<S::Value as Strategy>::Tree>>,
    // The final value to produce after successive calls to complicate() on the
    // underlying objects return false.
    final_complication: Option<Fuse<<S::Value as Strategy>::Tree>>,
    // When `simplify()` or `complicate()` causes a new `Strategy` to be
    // chosen, we need to find a new failing input for that case. To do this,
    // we implement `complicate()` by regenerating values up to a number of
    // times corresponding to the maximum number of test cases. A `simplify()`
    // which does not cause a new strategy to be chosen always resets
    // `complicate_regen_remaining` to 0.
    //
    // This does unfortunately depart from the direct interpretation of
    // simplify/complicate as binary search, but is still easier to think about
    // than other implementations of higher-order strategies.
    runner: TestRunner,
    complicate_regen_remaining: u32,
}

impl<S: ValueTree> Clone for FlattenOutsideFirstValueTree<S>
where
    S::Value: Strategy + Clone,
    S: Clone,
    <S::Value as Strategy>::Tree: Clone,
{
    fn clone(&self) -> Self {
        FlattenOutsideFirstValueTree {
            meta: self.meta.clone(),
            current: self.current.clone(),
            final_complication: self.final_complication.clone(),
            last_complication: self.last_complication.clone(),
            runner: self.runner.clone(),
            complicate_regen_remaining: self.complicate_regen_remaining,
        }
    }
}

impl<S: ValueTree> fmt::Debug for FlattenOutsideFirstValueTree<S>
where
    S::Value: Strategy,
    S: fmt::Debug,
    <S::Value as Strategy>::Tree: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FlattenValueTree")
            .field("meta", &self.meta)
            .field("current", &self.current)
            .field("last_complication", &self.last_complication)
            .field("final_complication", &self.final_complication)
            .field(
                "complicate_regen_remaining",
                &self.complicate_regen_remaining,
            )
            .finish()
    }
}

impl<S: ValueTree> FlattenOutsideFirstValueTree<S>
where
    S::Value: Strategy,
{
    fn new(runner: &mut TestRunner, meta: S) -> Result<Self, Reason> {
        let current = meta.current().new_tree(runner)?;
        Ok(FlattenOutsideFirstValueTree {
            meta: Fuse::new(meta),
            current: Fuse::new(current),
            last_complication: None,
            final_complication: None,
            // TODO: partial_clone would be better
            // runner: runner.partial_clone(),
            runner: runner.clone(),
            complicate_regen_remaining: 0,
        })
    }
}

impl<S: ValueTree> ValueTree for FlattenOutsideFirstValueTree<S>
where
    S::Value: Strategy,
    <S::Value as Strategy>::Tree: Clone,
{
    type Value = <S::Value as Strategy>::Value;

    fn current(&self) -> Self::Value {
        self.current.current()
    }

    fn simplify(&mut self) -> bool {
        self.current.disallow_complicate();

        if self.meta.simplify() {
            if let Ok(v) = self.meta.current().new_tree(&mut self.runner) {
                self.last_complication = Some(Fuse::new(v));
                mem::swap(
                    self.last_complication.as_mut().unwrap(),
                    &mut self.current,
                );
                self.complicate_regen_remaining = self.runner.config().cases;
                return true;
            } else {
                self.meta.disallow_simplify();
            }
        }

        self.complicate_regen_remaining = 0;
        let mut old_current = self.current.clone();
        old_current.disallow_simplify();

        if self.current.simplify() {
            self.last_complication = Some(old_current);
            true
        } else {
            false
        }
    }

    fn complicate(&mut self) -> bool {
        if self.complicate_regen_remaining > 0 {
            if self.runner.flat_map_regen() {
                self.complicate_regen_remaining -= 1;

                if let Ok(v) = self.meta.current().new_tree(&mut self.runner) {
                    self.current = Fuse::new(v);
                    return true;
                }
            } else {
                self.complicate_regen_remaining = 0;
            }
        }

        if self.meta.complicate() {
            if let Ok(v) = self.meta.current().new_tree(&mut self.runner) {
                self.current = Fuse::new(v);
                self.complicate_regen_remaining = self.runner.config().cases;
                return true;
            } else {
            }
        }

        if self.current.complicate() {
            return true;
        }

        if let Some(v) = self.last_complication.take() {
            self.current = v;
            true
        } else {
            false
        }
    }
}