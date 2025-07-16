# Matrix-Matrix Multiplication
# The function f takes as inputs the dimensions of input matrices, the MxK-matrix A and the KxN matrix B, and the MxN matrix C.
# f should store the matrix product A*B in the entries of C.
# row and entry are auxiliary functions used by f.
# Reminder: Matrix-Matrix Multiplication works as follows:
# Given matrix A of size MxK and matrix B of size KxN, their product is the matrix C of size MxN, where:
# C[i, j] = sum of { A[i, k] * B[k, j] for k in {0,...,p-1}}.
# Essentially, each entry of C is a piecewise product of the corresponding row of A and the corresponding column of B.

# Matrices are two-dimensional arrays
# M, N, K are the dimensions of the matrices A, B, and C
# A is an MxK matrix
# B is a  KxN matrix
# C is an MxN matrix

def entry(M, N, K, A, B, C, i, j, j_a=0, i_b=0):
    if i < M and j < N and j_a < K and i_b < K:
        C[i][j] += A[i][j_a] * B[i_b][j]
        entry(M, N, K, A, B, C, i+1, j+1, j_a, i_b)

def row(M, N, K, A, B, C, i, j=0):
    if i < M and j < N:
        entry(M, N, K, A, B, C, i, j)
        row(M, N, K, A, B, C, i, j+1)

def f(M, N, K, A, B, C, i=0):
    if i < M:
        row(M, N, K, A, B, C, i)
        f(M, N, K, A, B, C, i+1)
    return C
