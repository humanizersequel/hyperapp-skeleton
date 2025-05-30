# 📚 Complete Examples Reference

This guide provides complete, working examples of different types of Hyperware applications. Each example is self-contained and demonstrates specific patterns.

## Example 1: Todo List with P2P Sync

A collaborative todo list where items sync between nodes.

### Backend (lib.rs)
```rust
use hyperprocess_macro::*;
use hyperware_process_lib::{our, Address, ProcessId, Request, homepage::add_to_homepage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub completed: bool,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TodoState {
    todos: HashMap<String, TodoItem>,
    shared_with: Vec<String>, // Nodes we're sharing with
    sync_enabled: bool,
}

#[hyperprocess(
    name = "P2P Todo",
    ui = Some(HttpBindingConfig::default()),
    endpoints = vec![
        Binding::Http { 
            path: "/api", 
            config: HttpBindingConfig::new(false, false, false, None) 
        }
    ],
    save_config = SaveOptions::EveryMessage,
    wit_world = "todo-app-dot-os-v0"
)]
impl TodoState {
    #[init]
    async fn initialize(&mut self) {
        add_to_homepage("P2P Todo", Some("📝"), Some("/"), None);
        self.sync_enabled = true;
        println!("P2P Todo initialized on {}", our().node);
    }
    
    // Create a new todo
    #[http]
    async fn create_todo(&mut self, request_body: String) -> Result<String, String> {
        let text: String = serde_json::from_str(&request_body)
            .map_err(|_| "Invalid todo text".to_string())?;
        
        let todo = TodoItem {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            completed: false,
            created_by: our().node.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        let id = todo.id.clone();
        self.todos.insert(id.clone(), todo.clone());
        
        // Sync with peers
        if self.sync_enabled {
            self.broadcast_todo_update(todo).await;
        }
        
        Ok(serde_json::json!({ "id": id }).to_string())
    }
    
    // Get all todos
    #[http]
    async fn get_todos(&self, _request_body: String) -> String {
        let todos: Vec<&TodoItem> = self.todos.values().collect();
        serde_json::to_string(&todos).unwrap()
    }
    
    // Toggle todo completion
    #[http]
    async fn toggle_todo(&mut self, request_body: String) -> Result<String, String> {
        let id: String = serde_json::from_str(&request_body)
            .map_err(|_| "Invalid ID".to_string())?;
        
        if let Some(todo) = self.todos.get_mut(&id) {
            todo.completed = !todo.completed;
            todo.updated_at = chrono::Utc::now().to_rfc3339();
            
            if self.sync_enabled {
                self.broadcast_todo_update(todo.clone()).await;
            }
            
            Ok("Toggled".to_string())
        } else {
            Err("Todo not found".to_string())
        }
    }
    
    // Share with another node
    #[http]
    async fn share_with_node(&mut self, request_body: String) -> Result<String, String> {
        let node: String = serde_json::from_str(&request_body)?;
        
        if !self.shared_with.contains(&node) {
            self.shared_with.push(node.clone());
        }
        
        // Send initial sync
        self.sync_todos_with_node(node).await?;
        
        Ok("Shared successfully".to_string())
    }
    
    // Handle incoming todo updates
    #[remote]
    async fn receive_todo_update(&mut self, todo_json: String) -> Result<String, String> {
        let todo: TodoItem = serde_json::from_str(&todo_json)?;
        
        // Update or insert based on timestamp
        match self.todos.get(&todo.id) {
            Some(existing) if existing.updated_at < todo.updated_at => {
                self.todos.insert(todo.id.clone(), todo);
            }
            None => {
                self.todos.insert(todo.id.clone(), todo);
            }
            _ => {} // Our version is newer
        }
        
        Ok("ACK".to_string())
    }
    
    // Handle sync request
    #[remote]
    async fn handle_sync_request(&self, _request: String) -> Result<String, String> {
        let todos: Vec<&TodoItem> = self.todos.values().collect();
        Ok(serde_json::to_string(&todos).unwrap())
    }
    
    // Helper methods (in same impl block for hyperprocess)
    async fn broadcast_todo_update(&self, todo: TodoItem) {
        let wrapper = json!({
            "ReceiveTodoUpdate": serde_json::to_string(&todo).unwrap()
        });
        
        let process_id = "todo-app:todo-app:skeleton.os".parse::<ProcessId>().unwrap();
        
        for node in &self.shared_with {
            let target = Address::new(node.clone(), process_id.clone());
            let _ = Request::new()
                .target(target)
                .body(serde_json::to_vec(&wrapper).unwrap())
                .expects_response(5)
                .send();
        }
    }
    
    async fn sync_todos_with_node(&self, node: String) -> Result<(), String> {
        let process_id = "todo-app:todo-app:skeleton.os".parse::<ProcessId>()
            .map_err(|e| format!("Invalid process ID: {}", e))?;
        
        let target = Address::new(node, process_id);
        let wrapper = json!({ "HandleSyncRequest": "" });
        
        let response = Request::new()
            .target(target)
            .body(serde_json::to_vec(&wrapper).unwrap())
            .expects_response(10)
            .send_and_await_response(10)
            .map_err(|e| format!("Sync failed: {:?}", e))?;
        
        if let Ok(body) = response.body() {
            let remote_todos: Vec<TodoItem> = serde_json::from_slice(&body)?;
            // Merge logic would go here
            println!("Received {} todos from peer", remote_todos.len());
        }
        
        Ok(())
    }
}
```

### Frontend (App.tsx)
```typescript
import React, { useState, useEffect } from 'react';
import { create } from 'zustand';

// Types
interface TodoItem {
  id: string;
  text: string;
  completed: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

// API
const api = {
  async createTodo(text: string): Promise<{ id: string }> {
    const res = await fetch('/api', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ CreateTodo: text }),
    });
    return res.json();
  },

  async getTodos(): Promise<TodoItem[]> {
    const res = await fetch('/api', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ GetTodos: "" }),
    });
    return res.json();
  },

  async toggleTodo(id: string): Promise<void> {
    await fetch('/api', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ToggleTodo: id }),
    });
  },

  async shareWithNode(node: string): Promise<void> {
    await fetch('/api', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ShareWithNode: node }),
    });
  },
};

// Store
interface TodoStore {
  todos: TodoItem[];
  isLoading: boolean;
  fetchTodos: () => Promise<void>;
  createTodo: (text: string) => Promise<void>;
  toggleTodo: (id: string) => Promise<void>;
  shareWith: (node: string) => Promise<void>;
}

const useTodoStore = create<TodoStore>((set, get) => ({
  todos: [],
  isLoading: false,

  fetchTodos: async () => {
    set({ isLoading: true });
    try {
      const todos = await api.getTodos();
      set({ todos, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch todos:', error);
      set({ isLoading: false });
    }
  },

  createTodo: async (text: string) => {
    await api.createTodo(text);
    await get().fetchTodos();
  },

  toggleTodo: async (id: string) => {
    await api.toggleTodo(id);
    await get().fetchTodos();
  },

  shareWith: async (node: string) => {
    await api.shareWithNode(node);
  },
}));

// Components
export function TodoApp() {
  const { todos, isLoading, fetchTodos, createTodo, toggleTodo, shareWith } = useTodoStore();
  const [newTodo, setNewTodo] = useState('');
  const [shareNode, setShareNode] = useState('');

  useEffect(() => {
    fetchTodos();
    // Poll for updates
    const interval = setInterval(fetchTodos, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (newTodo.trim()) {
      await createTodo(newTodo.trim());
      setNewTodo('');
    }
  };

  const handleShare = async () => {
    if (shareNode.trim()) {
      try {
        await shareWith(shareNode);
        alert(`Shared with ${shareNode}`);
        setShareNode('');
      } catch (error) {
        alert(`Failed to share: ${error}`);
      }
    }
  };

  return (
    <div className="todo-app">
      <h1>P2P Todo List</h1>
      
      <div className="share-section">
        <input
          type="text"
          placeholder="Node to share with (e.g., bob.os)"
          value={shareNode}
          onChange={(e) => setShareNode(e.target.value)}
        />
        <button onClick={handleShare}>Share</button>
      </div>

      <form onSubmit={handleSubmit}>
        <input
          type="text"
          placeholder="Add a new todo..."
          value={newTodo}
          onChange={(e) => setNewTodo(e.target.value)}
        />
        <button type="submit">Add</button>
      </form>

      {isLoading && todos.length === 0 ? (
        <p>Loading...</p>
      ) : (
        <ul className="todo-list">
          {todos.map((todo) => (
            <li key={todo.id} className={todo.completed ? 'completed' : ''}>
              <input
                type="checkbox"
                checked={todo.completed}
                onChange={() => toggleTodo(todo.id)}
              />
              <span>{todo.text}</span>
              <small>by {todo.created_by}</small>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

---

## Example 2: Real-time Collaborative Notepad

A notepad where multiple users can edit simultaneously.

### Backend
```rust
use hyperprocess_macro::*;
use hyperware_process_lib::{our, Address, ProcessId, Request, homepage::add_to_homepage};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DocumentOp {
    pub id: String,
    pub op_type: OpType,
    pub position: usize,
    pub content: String,
    pub author: String,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OpType {
    Insert,
    Delete,
}

#[derive(Default, Serialize, Deserialize)]
pub struct NotepadState {
    document: String,
    operations: Vec<DocumentOp>,
    collaborators: Vec<String>,
    operation_counter: u64,
}

#[hyperprocess(
    name = "Collaborative Notepad",
    ui = Some(HttpBindingConfig::default()),
    endpoints = vec![
        Binding::Http { 
            path: "/api", 
            config: HttpBindingConfig::new(false, false, false, None) 
        }
    ],
    save_config = SaveOptions::OnInterval(10),
    wit_world = "notepad-dot-os-v0"
)]
impl NotepadState {
    #[init]
    async fn initialize(&mut self) {
        add_to_homepage("Notepad", Some("📄"), Some("/"), None);
        self.document = String::new();
        println!("Collaborative Notepad initialized");
    }
    
    // Get current document
    #[http]
    async fn get_document(&self, _request_body: String) -> String {
        serde_json::json!({
            "content": self.document,
            "collaborators": self.collaborators,
            "version": self.operation_counter,
        }).to_string()
    }
    
    // Apply local edit
    #[http]
    async fn edit_document(&mut self, request_body: String) -> Result<String, String> {
        #[derive(Deserialize)]
        struct EditRequest {
            op_type: String,
            position: usize,
            content: String,
        }
        
        let req: EditRequest = serde_json::from_str(&request_body)?;
        
        let op = DocumentOp {
            id: format!("{}-{}", our().node, self.operation_counter),
            op_type: match req.op_type.as_str() {
                "insert" => OpType::Insert,
                "delete" => OpType::Delete,
                _ => return Err("Invalid operation type".to_string()),
            },
            position: req.position,
            content: req.content,
            author: our().node.clone(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };
        
        // Apply operation locally
        self.apply_operation(&op)?;
        
        // Broadcast to collaborators
        self.broadcast_operation(op.clone()).await;
        
        Ok(serde_json::json!({ "version": self.operation_counter }).to_string())
    }
    
    // Join collaboration
    #[http]
    async fn join_collaboration(&mut self, request_body: String) -> Result<String, String> {
        let node: String = serde_json::from_str(&request_body)?;
        
        // Request current state from node
        let process_id = "notepad:notepad:skeleton.os".parse::<ProcessId>()?;
        let target = Address::new(node.clone(), process_id);
        
        let wrapper = json!({ "RequestState": our().node });
        
        let response = Request::new()
            .target(target)
            .body(serde_json::to_vec(&wrapper).unwrap())
            .expects_response(10)
            .send_and_await_response(10)?;
        
        if let Ok(body) = response.body() {
            let state: serde_json::Value = serde_json::from_slice(&body)?;
            self.document = state["document"].as_str().unwrap_or("").to_string();
            self.operation_counter = state["version"].as_u64().unwrap_or(0);
        }
        
        if !self.collaborators.contains(&node) {
            self.collaborators.push(node);
        }
        
        Ok("Joined".to_string())
    }
    
    // Handle incoming operations
    #[remote]
    async fn receive_operation(&mut self, op_json: String) -> Result<String, String> {
        let op: DocumentOp = serde_json::from_str(&op_json)?;
        
        // Check if we've already applied this operation
        if !self.operations.iter().any(|o| o.id == op.id) {
            self.apply_operation(&op)?;
            
            // Forward to other collaborators (gossip)
            self.broadcast_operation(op).await;
        }
        
        Ok("ACK".to_string())
    }
    
    // Handle state request
    #[remote]
    async fn request_state(&mut self, requester: String) -> Result<String, String> {
        if !self.collaborators.contains(&requester) {
            self.collaborators.push(requester);
        }
        
        Ok(serde_json::json!({
            "document": self.document,
            "version": self.operation_counter,
        }).to_string())
    }
    
    // Apply operation to document
    fn apply_operation(&mut self, op: &DocumentOp) -> Result<(), String> {
        match op.op_type {
            OpType::Insert => {
                if op.position <= self.document.len() {
                    self.document.insert_str(op.position, &op.content);
                } else {
                    return Err("Invalid position".to_string());
                }
            }
            OpType::Delete => {
                let end = (op.position + op.content.len()).min(self.document.len());
                self.document.replace_range(op.position..end, "");
            }
        }
        
        self.operations.push(op.clone());
        self.operation_counter += 1;
        
        // Limit operation history
        if self.operations.len() > 1000 {
            self.operations.drain(0..100);
        }
        
        Ok(())
    }
    
    // Broadcast operation to all collaborators
    async fn broadcast_operation(&self, op: DocumentOp) {
        let wrapper = json!({
            "ReceiveOperation": serde_json::to_string(&op).unwrap()
        });
        
        let process_id = "notepad:notepad:skeleton.os".parse::<ProcessId>().unwrap();
        
        for node in &self.collaborators {
            if node != &op.author { // Don't send back to author
                let target = Address::new(node.clone(), process_id.clone());
                let _ = Request::new()
                    .target(target)
                    .body(serde_json::to_vec(&wrapper).unwrap())
                    .expects_response(5)
                    .send();
            }
        }
    }
}
```

---

## Example 3: Distributed Key-Value Store

A simple distributed database with eventual consistency.

### Backend
```rust
use hyperprocess_macro::*;
use hyperware_process_lib::{our, Address, ProcessId, Request, homepage::add_to_homepage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct KVEntry {
    pub key: String,
    pub value: String,
    pub version: u64,
    pub updated_by: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ReplicationLog {
    pub entries: Vec<KVEntry>,
    pub from_node: String,
    pub timestamp: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct KVStore {
    data: HashMap<String, KVEntry>,
    replicas: Vec<String>,
    replication_enabled: bool,
    last_sync: HashMap<String, String>, // node -> timestamp
}

#[hyperprocess(
    name = "Distributed KV Store",
    ui = Some(HttpBindingConfig::default()),
    endpoints = vec![
        Binding::Http { 
            path: "/api", 
            config: HttpBindingConfig::new(false, false, false, None) 
        }
    ],
    save_config = SaveOptions::EveryMessage,
    wit_world = "kvstore-dot-os-v0"
)]
impl KVStore {
    #[init]
    async fn initialize(&mut self) {
        add_to_homepage("KV Store", Some("🗄️"), Some("/"), None);
        self.replication_enabled = true;
        
        // Start periodic sync
        self.schedule_periodic_sync();
    }
    
    // Set a key-value pair
    #[http]
    async fn set(&mut self, request_body: String) -> Result<String, String> {
        #[derive(Deserialize)]
        struct SetRequest {
            key: String,
            value: String,
        }
        
        let req: SetRequest = serde_json::from_str(&request_body)?;
        
        let entry = KVEntry {
            key: req.key.clone(),
            value: req.value,
            version: self.data.get(&req.key).map(|e| e.version + 1).unwrap_or(1),
            updated_by: our().node.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        self.data.insert(req.key.clone(), entry.clone());
        
        // Replicate to other nodes
        if self.replication_enabled {
            self.replicate_entry(entry).await;
        }
        
        Ok("OK".to_string())
    }
    
    // Get a value by key
    #[http]
    async fn get(&self, request_body: String) -> Result<String, String> {
        let key: String = serde_json::from_str(&request_body)?;
        
        match self.data.get(&key) {
            Some(entry) => Ok(serde_json::json!({
                "value": entry.value,
                "version": entry.version,
                "updated_by": entry.updated_by,
                "updated_at": entry.updated_at,
            }).to_string()),
            None => Err("Key not found".to_string()),
        }
    }
    
    // List all keys
    #[http]
    async fn list_keys(&self, _request_body: String) -> String {
        let keys: Vec<String> = self.data.keys().cloned().collect();
        serde_json::to_string(&keys).unwrap()
    }
    
    // Add a replica node
    #[http]
    async fn add_replica(&mut self, request_body: String) -> Result<String, String> {
        let node: String = serde_json::from_str(&request_body)?;
        
        if !self.replicas.contains(&node) {
            self.replicas.push(node.clone());
            
            // Send initial sync
            self.sync_with_replica(node).await?;
        }
        
        Ok("Replica added".to_string())
    }
    
    // Handle incoming replication
    #[remote]
    async fn replicate(&mut self, entry_json: String) -> Result<String, String> {
        let entry: KVEntry = serde_json::from_str(&entry_json)?;
        
        // Apply if newer
        match self.data.get(&entry.key) {
            Some(existing) if existing.version < entry.version => {
                self.data.insert(entry.key.clone(), entry);
            }
            None => {
                self.data.insert(entry.key.clone(), entry);
            }
            _ => {} // Our version is newer
        }
        
        Ok("ACK".to_string())
    }
    
    // Handle sync request
    #[remote]
    async fn sync(&mut self, since_json: String) -> Result<String, String> {
        let since: String = serde_json::from_str(&since_json)?;
        
        let entries: Vec<KVEntry> = self.data.values()
            .filter(|e| e.updated_at > since)
            .cloned()
            .collect();
        
        let log = ReplicationLog {
            entries,
            from_node: our().node.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        
        Ok(serde_json::to_string(&log).unwrap())
    }
    
    // Replicate a single entry
    async fn replicate_entry(&self, entry: KVEntry) {
        let wrapper = json!({
            "Replicate": serde_json::to_string(&entry).unwrap()
        });
        
        let process_id = "kvstore:kvstore:skeleton.os".parse::<ProcessId>().unwrap();
        
        for replica in &self.replicas {
            let target = Address::new(replica.clone(), process_id.clone());
            let _ = Request::new()
                .target(target)
                .body(serde_json::to_vec(&wrapper).unwrap())
                .expects_response(5)
                .send();
        }
    }
    
    // Sync with a specific replica
    async fn sync_with_replica(&mut self, node: String) -> Result<(), String> {
        let last_sync = self.last_sync.get(&node)
            .cloned()
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        
        let process_id = "kvstore:kvstore:skeleton.os".parse::<ProcessId>()?;
        let target = Address::new(node.clone(), process_id);
        
        let wrapper = json!({ "Sync": last_sync });
        
        let response = Request::new()
            .target(target)
            .body(serde_json::to_vec(&wrapper).unwrap())
            .expects_response(10)
            .send_and_await_response(10)?;
        
        if let Ok(body) = response.body() {
            let log: ReplicationLog = serde_json::from_slice(&body)?;
            
            for entry in log.entries {
                match self.data.get(&entry.key) {
                    Some(existing) if existing.version < entry.version => {
                        self.data.insert(entry.key.clone(), entry);
                    }
                    None => {
                        self.data.insert(entry.key.clone(), entry);
                    }
                    _ => {}
                }
            }
            
            self.last_sync.insert(node, log.timestamp);
        }
        
        Ok(())
    }
    
    // Schedule periodic sync
    fn schedule_periodic_sync(&self) {
        // In real implementation, use timer API
        // timer::set_timer(60000, Some(json!({ "action": "sync" })));
    }
}
```

---

## Example 4: P2P File Sharing

Share files directly between nodes.

### Backend Snippet
```rust
#[derive(Serialize, Deserialize)]
pub struct SharedFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub owner: String,
    pub shared_with: Vec<String>,
    pub uploaded_at: String,
}

#[http]
async fn upload_file(&mut self, name: String, mime_type: String, data: Vec<u8>) -> Result<String, String> {
    let file_id = uuid::Uuid::new_v4().to_string();
    let file_path = format!("/fileshare:{}/files/{}", our().node, file_id);
    
    // Store in VFS
    let vfs_address = Address::new(our().node.clone(), "vfs:distro:sys".parse::<ProcessId>().unwrap());
    
    let write_request = json!({
        "path": file_path,
        "action": "Write"
    });
    
    Request::new()
        .target(vfs_address)
        .body(serde_json::to_vec(&write_request).unwrap())
        .blob(LazyLoadBlob::new(Some("file"), data.clone()))
        .expects_response(5)
        .send_and_await_response(5)?;
    
    let file = SharedFile {
        id: file_id.clone(),
        name,
        size: data.len() as u64,
        mime_type,
        owner: our().node.clone(),
        shared_with: vec![],
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };
    
    self.files.insert(file_id.clone(), file);
    Ok(file_id)
}

#[http]
async fn share_file(&mut self, request_body: String) -> Result<String, String> {
    #[derive(Deserialize)]
    struct ShareRequest {
        file_id: String,
        node: String,
    }
    
    let req: ShareRequest = serde_json::from_str(&request_body)?;
    
    if let Some(file) = self.files.get_mut(&req.file_id) {
        if !file.shared_with.contains(&req.node) {
            file.shared_with.push(req.node.clone());
        }
        
        // Notify the node
        let process_id = "fileshare:fileshare:skeleton.os".parse::<ProcessId>()?;
        let target = Address::new(req.node, process_id);
        
        let notification = json!({
            "FileSharedWithYou": serde_json::to_string(&file).unwrap()
        });
        
        Request::new()
            .target(target)
            .body(serde_json::to_vec(&notification).unwrap())
            .expects_response(5)
            .send()?;
        
        Ok("Shared".to_string())
    } else {
        Err("File not found".to_string())
    }
}

#[remote]
async fn request_file(&self, file_id: String) -> Result<Vec<u8>, String> {
    // Check if requester has access
    let requester = our().source.node.clone();
    
    if let Some(file) = self.files.get(&file_id) {
        if file.owner != our().node && !file.shared_with.contains(&requester) {
            return Err("Access denied".to_string());
        }
        
        // Read from VFS
        let file_path = format!("/fileshare:{}/files/{}", our().node, file_id);
        let vfs_address = Address::new(our().node.clone(), "vfs:distro:sys".parse::<ProcessId>().unwrap());
        
        let read_request = json!({
            "path": file_path,
            "action": "Read"
        });
        
        let response = Request::new()
            .target(vfs_address)
            .body(serde_json::to_vec(&read_request).unwrap())
            .expects_response(5)
            .send_and_await_response(5)?;
        
        if let Some(blob) = response.blob() {
            Ok(blob.bytes)
        } else {
            Err("File data not found".to_string())
        }
    } else {
        Err("File not found".to_string())
    }
}
```

---

## Tips for Building Your Own Apps

### 1. Start with the Skeleton
```bash
cp -r hyperapp-skeleton myapp
cd myapp
# Update metadata.json with your app name
# Modify skeleton-app to match your app name
```

### 2. Common Modifications

#### Change App Name
1. Update `metadata.json`
2. Update `Cargo.toml` (both workspace and app)
3. Rename `skeleton-app` directory
4. Update imports and ProcessId strings

#### Add Dependencies
```toml
# In your-app/Cargo.toml
[dependencies]
# ... existing deps
uuid = { version = "1.4.1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

#### Add Complex State
```rust
// Use JSON for complex internal state
#[derive(Default, Serialize, Deserialize)]
pub struct AppState {
    // Simple types for WIT
    pub item_count: u32,
    pub last_update: String,
    
    // Complex types as JSON
    #[serde(skip)]
    complex_data: HashMap<String, ComplexType>,
}

impl AppState {
    fn save_complex_data(&mut self) {
        // Serialize to JSON when needed
        self.complex_json = serde_json::to_string(&self.complex_data).unwrap();
    }
}
```

### 3. Testing Your App

#### Local Testing
```bash
# Build
kit b --hyperapp

# Run
kit s

# Check logs
# Backend logs appear in terminal
# Frontend logs in browser console
```

#### P2P Testing
```bash
# Terminal 1
kit s --fake-node alice.os

# Terminal 2
kit s --fake-node bob.os --port 8081

# Test communication between nodes
```

### 4. Common Gotchas

1. **Always** include `_request_body` in HTTP handlers
2. **Always** use tuple format for multi-param calls
3. **Always** set timeout on remote requests
4. **Never** forget the `/our.js` script
5. **Test** P2P features with multiple nodes early

## Remember

These examples show patterns, not prescriptions. Adapt them to your needs:
- Simplify for single-node apps
- Add complexity for advanced features
- Mix patterns as needed
- Keep security in mind
- Design for offline-first
- Test edge cases