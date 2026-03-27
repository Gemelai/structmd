# Lists Schema

```bnf
document ::= H1("Config") entry+
entry    ::= H2(IDENTIFIER) property+
```

```types
@mounts : list(string)
@tags   : list(string)
```
