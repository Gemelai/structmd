# Schema

```bnf
document ::= H1("Tasks") task+
task     ::= H2(IDENTIFIER) property+
```

```types
@schedule : cron, required
@run      : string, required
```
