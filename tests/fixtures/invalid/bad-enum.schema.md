# Schema

```bnf
document ::= H1("Config") item+
item     ::= H2(IDENTIFIER) property+
```

```types
@color : enum(red, green, blue), required
```
