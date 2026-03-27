# Optional H1 Schema

```bnf
document ::= main extras?
main     ::= H1("Main") item+
item     ::= H2(IDENTIFIER) property+
extras   ::= H1("Extras") item+
```

```types
@key : string
```
