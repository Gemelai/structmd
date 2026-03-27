# Simple Schema

```bnf
document ::= H1("Config") item+
item     ::= H2(IDENTIFIER) property+
```

```types
@name  : string, required
@count : integer
```
