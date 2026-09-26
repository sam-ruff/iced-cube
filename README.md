# iced-cube

Components for [iced](https://iced.rs) applications: buttons, inputs, selection controls, overlays, toasts, tabs and more, all drawn from one set of theme tokens so they look right together in light and dark mode. Icons come from [Lucide](https://lucide.dev) and are resolved at compile time.

Docs with live examples: https://sam-ruff.github.io/iced-cube/

```sh
cargo add iced@0.14 iced-cube
```

```rust
use iced::Element;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone)]
enum Message {
    Save,
}

fn view() -> Element<'static, Message> {
    button("Save")
        .icon(lucide!(Save))
        .variant(Variant::Primary)
        .on_press(Message::Save)
        .into()
}
```

Components never own your state. Anything stateful, such as tabs, accordions or the toast queue, gives you a plain struct with a pure `update` that you store and pass back in. Components that take data from background work do it through a subscription that owns a channel, so nothing is shared behind a lock.

## Running the examples

```sh
cargo run -p gallery
```

The gallery is the same code that runs in the browser on the docs site.

## Status

Early days. The API will change before 1.0, and minor releases may include breaking changes until then.

## Licence

MIT or Apache-2.0, at your option. Lucide icons are ISC licensed; see `LICENSE-LUCIDE`.
