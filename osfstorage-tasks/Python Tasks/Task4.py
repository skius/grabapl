# Smallest Distance Between Consecutive Integers
# The function f should return the smallest distance of two consecutive integers (in ascending order) in the list.
# For example, [1, 2, 3] would return 1, and [3, 2, 1, 4] would return 3.
# f uses an auxiliary function aux.

INFTY = 1000

def aux(l, d, v):
    if len(l) == 0:
        return INFTY
    if l[0] == v:
        return d
    return aux(l[1:], d + 1, v)

def f(l):
    if len(l) == 0:
        return INFTY
    v1 = aux(l[1:], l[0]+1, 1)
    v2 = f(l[1:])
    return min(v1, v2)
