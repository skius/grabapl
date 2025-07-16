# Greatest Common Divisor
# The function f should return the greatest common divisor of a and b.
# Reminder: The GCD of two numbers a and b is the greatest number c that divides both a and b.
# For example, the GCD of 12 and 8 is 4.

def f(a, b):
    if b != 0:
        tmp = b
        b = b % a
        a = tmp
        return f(a, b)
    else:
        return a
