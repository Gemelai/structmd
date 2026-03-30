# Scoped Types Schema

```grammar
document  ::= H1("Agyo") container servers
container ::= H2("Container") property+
servers   ::= H2("Servers") server+
server    ::= H3(IDENTIFIER) property+
```

```types:container
@image   : string, required
@network : string, default("outbound")
```

```types:server
@command : string, required
```
