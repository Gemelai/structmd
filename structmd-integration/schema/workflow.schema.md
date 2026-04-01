# Workflow Schema

A workflow is a sequence of named steps. Each step has a shell command and
optional dependencies on other steps by name.

```grammar
document ::= H1 step+
step     ::= H2(SNAKE_CASE) property+
```

```types
@command : string, required
@depends : list(string)
```
