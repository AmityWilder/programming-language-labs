# Variable

## Declaration, Access, and Assignment

### Variable access before declaration

Static analysis error. If a variable is accessed before declaration or in a wider scope than the one in which it was declared, static analysis produces an "unknown variable" error.

**Example:**

```
error: cannot find variable within this scope
  --> example\var-early-access.bats:1:6-7
  |
1 |  prnt(x); // expect "unknown variable" error
  |       ^ unknown variable
  |
help: check if the variable is in a tighter scope than where you are trying to use it
```

### Missing initializer in declaration

### Redeclaration
