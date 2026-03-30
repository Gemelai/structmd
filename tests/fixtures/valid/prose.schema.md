# Prose Schema

```grammar
document ::= H1("Tools") tool+
tool     ::= H2(IDENTIFIER) prose property+
```

```types
@server : string, required
```
