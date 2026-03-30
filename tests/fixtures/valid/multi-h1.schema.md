# Multi-H1 Schema

```grammar
document ::= settings items
settings ::= H1("Settings") property+
items    ::= H1("Items") item+
item     ::= H2(IDENTIFIER) property+
```

```types:settings
@path : string, required
```

```types:item
@enabled : bool
```
