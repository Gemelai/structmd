# Schema

```grammar
document ::= H1("Root") alpha beta
alpha    ::= H2("Alpha") property+
beta     ::= H2("Beta") property+
```

```types:alpha
@color : enum(red, green, blue), required
```

```types:beta
@size : integer, required
```
