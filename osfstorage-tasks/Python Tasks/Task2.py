# Max Heap Removal
# The function f should take as input the root note of a max-heap, and
# it should return the maximum of the heap (root node), and
# then restore the heap condition.
# Reminder: A maximum heap is a binary tree in which the number value of each node is greater than the number value of its children, and
# each node in the tree is a maximum heap itself.

class Node:
    def __init__(self, value, left=None, right=None):
        self.value = value
        self.left = left
        self.right = right
    def __str__(self):
        if not self.left and not self.right:
            return f'{self.value}'
        return f'({self.value}: {self.left or ""},{self.right or ""})'
    def __eq__(self, other):
        if other is None: return False
        if self.value != other.value: return False
        cs = [c for c in [self.left, self.right] if c]
        co = [c for c in [other.left, other.right] if c]
        return len(cs) == len(co) and all([a == b for (a, b) in zip(cs, co)])

# for the sake of simplicity, assume all nodes in this heap either have 0 or 2 children
def f(node):
    if not node.left and not node.right:
        return node.value, None
    else:
        if node.left.value > node.right.value:
            value, left = f(node.left)
            return node.value, Node(value, left, node.left)
        else:
            value, right = f(node.right)
            return node.value, Node(value, node.right, right)
