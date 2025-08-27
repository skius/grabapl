# Design

## Requirements

### Graphs
* Directed
* No self-loops
* Arbitrary node attributes
* Arbitrary edge attributes
* Edges are ordered
  * One order for all incoming edges of a node
  * One order for all outgoing edges of a node

### Classical programming language analogy

#### Values
Values are the dynamic, concrete state of a program.
Examples include
* Concrete strings: "abc", "hello", ...
* Concrete numbers: 1, 2, 3, ...
* Concrete booleans: true, false

These are only known dynamically, so they cannot be used to statically reason about the program.

In grabapl, values take form of a *value* graph, where each node's and each edge's attribute is a value.

#### Types
Types are the static, abstract description of a program state.

Traditionally, each type can be represented as a set of values.
For example:
* String = {"abc", "hello", ...}
* Number = {1, 2, 3, ...}
* Boolean = {true, false}
* Top = {"abc", 1, false, ...}

Types are known statically.

**Subtyping** is the notion of set containment expressed in terms of types.

Example:
* String/Number/Boolean are subtypes of Top

In grabapl, types are represented as *type* graphs, where each node's and each edge's attribute is a type.

### Functions ==> Operations and Queries
In classical, type-checked PL, functions are defined for some parameter types.

Functions can be called with argument types if the argument types are subtypes of the parameter types.
This is pattern matching.

Assuming the argument types match, the function can be called. This is determined statically.

Then, dynamically, the *values* of the arguments get bound to the parameters, at which point the function is ran
concretely.

## Design
### Operations and Queries

There are two phases: type-checking with parameter binding, and concrete execution.

### Type-checking, pattern matching, parameter binding
The inputs to this phase are the following:
* An operation to execute
  * 
* A *type* graph
  * A graph where node/edge attributes are types 