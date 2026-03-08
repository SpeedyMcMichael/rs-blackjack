# blackjack-rs

terminal blackjack built with [ratatui](https://github.com/ratatui-org/ratatui)

```
┌─────┐ ┌─────┐
│A    │ │K    │
│  ♠  │ │  ♥  │
│    A│ │    K│
└─────┘ └─────┘
```

## install

```bash
git clone https://github.com/SpeedyMcMichael/blackjack-rs.git
# run
cd blackjack-rs
cargo run --release
```
or with the wrapper
```bash
git clone https://github.com/SpeedyMcMichael/rs-blackjack.git
# run
./blackjack.sh
```

## controls

| key | action |
|-----|--------|
| `0-9` | type bet |
| `Enter` | deal |
| `H` | hit |
| `S` | stand |
| `N` | new hand |
| `Q` | quit |

## rules

- start with 500 chips
- blackjack pays 1.5x
- dealer stands on soft 17

## built with

- [ratatui](https://github.com/ratatui-org/ratatui) — TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal backend
- [rand](https://github.com/rust-random/rand) — shuffling
- [love](https://en.wikipedia.org/wiki/Love) — motivator 
