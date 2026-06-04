# Design Decisions - ListenerStatus Serde Round-trip

- **Serde Attribute vs Custom Deserializer**: We will use `#[serde(rename_all = "lowercase")]` rather than a hand-rolled Deserialize implementation because it is standard, less prone to human error, and works out of the box with serde.
