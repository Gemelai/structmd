# Nested Schema

```grammar
document ::= H1("Root") parent
parent   ::= H2("Parent") child+
child    ::= H3(IDENTIFIER) property+
```

```types:child
@cmd     : string, required
@verbose : bool, default(false)
```
