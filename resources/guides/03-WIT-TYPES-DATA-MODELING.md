# 📊 WIT Types & Data Modeling Guide

## Understanding WIT (WebAssembly Interface Types)

WIT is the type system that bridges your Rust code with the frontend. The hyperprocess macro automatically generates WIT files from your Rust types, but it has strict requirements.

## Type Compatibility Matrix

| Rust Type | WIT Type | Supported | Notes |
|-----------|----------|-----------|-------|
| `bool` | `bool` | ✅ | |
| `u8`, `u16`, `u32`, `u64` | `u8`, `u16`, `u32`, `u64` | ✅ | |
| `i8`, `i16`, `i32`, `i64` | `s8`, `s16`, `s32`, `s64` | ✅ | |
| `f32`, `f64` | `float32`, `float64` | ✅ | |
| `String` | `string` | ✅ | |
| `Vec<T>` | `list<T>` | ✅ | T must be supported |
| `Option<T>` | `option<T>` | ✅ | T must be supported |
| `(T1, T2, ...)` | `tuple<T1, T2, ...>` | ✅ | All T must be supported |
| `HashMap<K, V>` | - | ❌ | Use `Vec<(K, V)>` |
| `HashSet<T>` | - | ❌ | Use `Vec<T>` |
| `[T; N]` | - | ❌ | Use `Vec<T>` |
| `&str` | - | ❌ | Use `String` |
| `&[T]` | - | ❌ | Use `Vec<T>` |
| Complex enums | - | ⚠️ | Only simple variants |
| Trait objects | - | ❌ | Not supported |

## Data Modeling Strategies

### 1. Simple Types - Direct Mapping

```rust
// ✅ These types map directly to WIT
#[derive(Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
    pub age: u32,
    pub active: bool,
    pub balance: f64,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Response {
    pub users: Vec<User>,
    pub total: u64,
    pub page: Option<u32>,
}

// Use in endpoint
#[http]
async fn get_users(&self, _request_body: String) -> Response {
    Response {
        users: self.users.clone(),
        total: self.users.len() as u64,
        page: Some(0),
    }
}
```

### 2. Complex Types - JSON String Pattern

```rust
// Internal complex type (not exposed via WIT)
#[derive(Serialize, Deserialize)]
struct ComplexGameState {
    board: HashMap<Position, Piece>,
    history: Vec<Move>,
    timers: HashMap<PlayerId, Duration>,
    metadata: serde_json::Value,
}

// ✅ Return as JSON string
#[http]
async fn get_game_state(&self, _request_body: String) -> String {
    serde_json::to_string(&self.game_state).unwrap()
}

// ✅ Accept as JSON string
#[http]
async fn update_game_state(&mut self, request_body: String) -> Result<String, String> {
    let state: ComplexGameState = serde_json::from_str(&request_body)
        .map_err(|e| format!("Invalid game state: {}", e))?;
    
    self.game_state = state;
    Ok("Updated".to_string())
}
```

### 3. Enum Handling

```rust
// ❌ WRONG - Complex enum variants not supported
pub enum GameEvent {
    PlayerJoined { player_id: String, timestamp: u64 },
    MoveMade { from: Position, to: Position },
    GameEnded { winner: Option<String>, reason: EndReason },
}

// ✅ PATTERN 1: Simple enum + data struct
#[derive(Serialize, Deserialize, PartialEq)]
pub enum EventType {
    PlayerJoined,
    MoveMade,
    GameEnded,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameEvent {
    pub event_type: EventType,
    pub player_id: Option<String>,
    pub from_position: Option<Position>,
    pub to_position: Option<Position>,
    pub winner: Option<String>,
    pub timestamp: u64,
}

// ✅ PATTERN 2: Tagged unions via JSON
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GameEvent {
    PlayerJoined { player_id: String, timestamp: u64 },
    MoveMade { from: Position, to: Position },
    GameEnded { winner: Option<String> },
}

// Return as JSON string
#[http]
async fn get_events(&self, _request_body: String) -> String {
    serde_json::to_string(&self.events).unwrap()
}
```

### 4. HashMap Replacement Patterns

```rust
// ❌ WRONG - HashMap not supported
pub struct GameData {
    pub players: HashMap<String, Player>,
    pub scores: HashMap<String, u32>,
}

// ✅ PATTERN 1: Use Vec of tuples
#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    pub players: Vec<(String, Player)>,
    pub scores: Vec<(String, u32)>,
}

// ✅ PATTERN 2: Separate key-value struct
#[derive(Serialize, Deserialize, PartialEq)]
pub struct PlayerEntry {
    pub id: String,
    pub player: Player,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ScoreEntry {
    pub player_id: String,
    pub score: u32,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameData {
    pub players: Vec<PlayerEntry>,
    pub scores: Vec<ScoreEntry>,
}

// ✅ PATTERN 3: Internal HashMap, external Vec
#[derive(Default, Serialize, Deserialize)]
pub struct AppState {
    // Internal representation (not exposed)
    players_map: HashMap<String, Player>,
}

// Exposed via endpoints
#[http]
async fn get_players(&self, _request_body: String) -> Vec<Player> {
    self.players_map.values().cloned().collect()
}

#[http]
async fn get_player(&self, request_body: String) -> Result<Player, String> {
    let id: String = serde_json::from_str(&request_body)?;
    self.players_map.get(&id)
        .cloned()
        .ok_or_else(|| "Player not found".to_string())
}
```

### 5. Nested Type Visibility

```rust
// ❌ PROBLEM: WIT generator can't find NestedData
pub struct Response {
    pub data: NestedData,
}

pub struct NestedData {
    pub items: Vec<Item>,
}

pub struct Item {
    pub id: String,
}

// ✅ FIX 1: Ensure all types are referenced in endpoints
#[http]
async fn get_response(&self, _request_body: String) -> Response { ... }

#[http]
async fn get_nested_data(&self, _request_body: String) -> NestedData { ... }

#[http]
async fn get_item(&self, _request_body: String) -> Item { ... }

// ✅ FIX 2: Flatten the structure
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Response {
    pub items: Vec<Item>,
    pub metadata: ResponseMetadata,
}
```

## Design Patterns for Data Modeling

### 1. Command Pattern for Complex Operations

```rust
// Instead of complex parameters, use command objects
#[derive(Deserialize)]
pub struct CreateGameCommand {
    pub name: String,
    pub max_players: u8,
    pub settings: GameSettings,
}

#[derive(Deserialize)]
pub struct GameSettings {
    pub time_limit: Option<u32>,
    pub allow_spectators: bool,
    pub game_mode: String,
}

#[http]
async fn create_game(&mut self, request_body: String) -> Result<String, String> {
    let command: CreateGameCommand = serde_json::from_str(&request_body)?;
    
    // Process command
    let game_id = self.create_game_from_command(command)?;
    
    Ok(serde_json::json!({ "game_id": game_id }).to_string())
}
```

### 2. View Pattern for Complex Queries

```rust
// Internal complex state
struct Game {
    id: String,
    players: HashMap<String, Player>,
    board: BoardState,
    history: Vec<Move>,
    // ... many more fields
}

// Simplified view for API
#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameView {
    pub id: String,
    pub player_count: u8,
    pub current_turn: String,
    pub status: GameStatus,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameDetailView {
    pub id: String,
    pub players: Vec<PlayerView>,
    pub board_state: String, // Serialized board
    pub last_move: Option<MoveView>,
}

// Expose views, not internal state
#[http]
async fn list_games(&self, _request_body: String) -> Vec<GameView> {
    self.games.values()
        .map(|game| game.to_view())
        .collect()
}

#[http]
async fn get_game_detail(&self, request_body: String) -> Result<GameDetailView, String> {
    let id: String = serde_json::from_str(&request_body)?;
    self.games.get(&id)
        .map(|game| game.to_detail_view())
        .ok_or_else(|| "Game not found".to_string())
}
```

### 3. Event Sourcing Pattern

```rust
// Events as simple data
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub data: String, // JSON encoded event data
}

// Store events, rebuild state
#[derive(Default, Serialize, Deserialize)]
pub struct AppState {
    events: Vec<Event>,
    // Cached current state (rebuilt from events)
    #[serde(skip)]
    current_state: Option<ComputedState>,
}

impl AppState {
    fn rebuild_state(&mut self) {
        let mut state = ComputedState::default();
        for event in &self.events {
            state.apply_event(event);
        }
        self.current_state = Some(state);
    }
}

#[http]
async fn add_event(&mut self, request_body: String) -> Result<String, String> {
    let event: Event = serde_json::from_str(&request_body)?;
    self.events.push(event);
    self.rebuild_state();
    Ok("Event added".to_string())
}
```

## Best Practices

### 1. Always Add PartialEq

```rust
// WIT-exposed types need PartialEq
#[derive(Serialize, Deserialize, PartialEq)]
pub struct MyType {
    pub field: String,
}
```

### 2. Use Builder Pattern for Complex Types

```rust
#[derive(Default)]
pub struct GameBuilder {
    name: Option<String>,
    max_players: Option<u8>,
    settings: GameSettings,
}

impl GameBuilder {
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    pub fn max_players(mut self, max: u8) -> Self {
        self.max_players = Some(max);
        self
    }
    
    pub fn build(self) -> Result<Game, String> {
        Ok(Game {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name.ok_or("Name required")?,
            max_players: self.max_players.unwrap_or(4),
            settings: self.settings,
            // ... initialize other fields
        })
    }
}
```

### 3. Version Your Data Models

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub data: serde_json::Value,
}

impl SaveData {
    pub fn migrate(self) -> Result<CurrentData, String> {
        match self.version {
            1 => migrate_v1_to_v2(self.data),
            2 => Ok(serde_json::from_value(self.data)?),
            _ => Err(format!("Unknown version: {}", self.version)),
        }
    }
}
```

### 4. Document Your Types

```rust
/// Represents a player in the game
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Player {
    /// Unique identifier for the player
    pub id: String,
    
    /// Display name chosen by the player
    pub name: String,
    
    /// Current score in the game
    pub score: u32,
    
    /// Whether the player is currently active
    pub active: bool,
}
```

## Common Patterns Reference

### Pattern 1: ID-based Lookups
```rust
// Store as HashMap internally, expose as list
pub struct AppState {
    items_map: HashMap<String, Item>,
}

#[http]
async fn get_item(&self, request_body: String) -> Result<Item, String> {
    let id: String = serde_json::from_str(&request_body)?;
    self.items_map.get(&id).cloned()
        .ok_or_else(|| "Not found".to_string())
}

#[http]
async fn list_items(&self, _request_body: String) -> Vec<Item> {
    self.items_map.values().cloned().collect()
}
```

### Pattern 2: Pagination
```rust
#[derive(Deserialize)]
pub struct PageRequest {
    pub page: usize,
    pub per_page: usize,
}

#[derive(Serialize, PartialEq)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[http]
async fn list_paginated(&self, request_body: String) -> PageResponse<Item> {
    let req: PageRequest = serde_json::from_str(&request_body)
        .unwrap_or(PageRequest { page: 0, per_page: 20 });
    
    let start = req.page * req.per_page;
    let items: Vec<_> = self.items
        .iter()
        .skip(start)
        .take(req.per_page)
        .cloned()
        .collect();
    
    PageResponse {
        items,
        total: self.items.len(),
        page: req.page,
        per_page: req.per_page,
    }
}
```

### Pattern 3: Result Types
```rust
#[derive(Serialize, Deserialize, PartialEq)]
pub struct ApiResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[http]
async fn safe_operation(&mut self, request_body: String) -> ApiResult<String> {
    match self.do_operation(request_body) {
        Ok(result) => ApiResult::ok(result),
        Err(e) => ApiResult::err(e.to_string()),
    }
}
```

## Remember

1. **When in doubt, use JSON strings** - They always work
2. **All public fields** - WIT needs to see them
3. **Test incrementally** - Build often to catch type issues early
4. **Keep it simple** - Complex types cause problems
5. **Document patterns** - Future you will thank you