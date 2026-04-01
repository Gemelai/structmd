# build pipeline

## fetch
- command: echo "fetching"

## compile
- command: echo "compiling"
- depends: fetch

## test
- command: echo "testing"
- depends: compile

## report
- command: echo "done"
- depends: test
