# Tables Schema

```bnf
document ::= H1("Tools") tool+
tool     ::= H2(SNAKE_CASE) prose property* table
```

```types
@server     : string
@parameters : label
```

```table
@name        : string
@type        : enum(string, integer, number, boolean, array, object)
@required    : enum(yes, no)
@description : string
```
