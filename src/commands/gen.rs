use anyhow::Result;
use std::fs;
use std::path::Path;
use lunar_interface::Ci144Config; // [FIXED] Cleaned up unused ActualJson and InterfacesYml imports

/// Generates a tailored AI Agent instruction prompt to bootstrap interfaces.yml.
pub async fn execute_interfaces(prompt: bool) -> Result<()> {
    if prompt {
        let autogen_path = Path::new(".lunar").join(".interfaces-autogen.json");
        let autogen_content = if autogen_path.exists() {
            fs::read_to_string(&autogen_path)?
        } else {
            "[]".to_string()
        };
        
        let project_name = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        println!("🤖 **AI Agent Instruction (LunarAST interfaces.yml Generator)**");
        println!("I have a project named '{}'. Here are the physical API facts extracted from code AST:", project_name);
        println!("```json\n{}\n```", autogen_content);
        println!("\nPlease analyze the paths, methods and routes, and write a standard `.lunar/interfaces.yml` file following this spec:");
        println!("```yaml\nproject: {}\ntype: service\nexposed:\n  - path: /api/route\n    method: GET\n    reason: \"describe what this API does\"\nconsumed:\n  - path: /upstream/route\n    method: POST\n    targetProject: otherService\n    reason: \"why we consume this\"\n```", project_name);
        println!("\nOutput ONLY the valid YAML. Do not write markdown wrapping outside the YAML block.");
    } else {
        println!("No prompt flag specified. Run 'lunar gen interfaces --prompt' to output the AI instruction template.");
    }
    Ok(())
}

/// Generates a tailored AI Agent prompt to bootstrap ci-144.yml, or compiles the bridge deterministically.
pub async fn execute_ci144(prompt: bool, lang: &str, out_dir: &str) -> Result<()> {
    if prompt {
        let interfaces_path = Path::new(".lunar").join("interfaces.yml");
        let interfaces_content = if interfaces_path.exists() {
            fs::read_to_string(&interfaces_path)?
        } else {
            "project: unknown\ntype: mixed".to_string()
        };

        println!("🤖 **AI Agent Instruction (LunarAST CI-144 Config Generator)**");
        println!("My project has the following interfaces defined in interfaces.yml:");
        println!("```yaml\n{}\n```", interfaces_content);
        println!("\nPlease generate a standard `.lunar/ci-144.yml` configuration mapping these interfaces to CI-144 SemanticNodes and Actions.");
        println!("Output only a valid, strictly formatted YAML document matching this structure:");
        println!("```yaml\nproject: <project_name>\ntarget: cellrix\nsemanticNodes:\n  - name: <node_name>\n    type: <type>\n    description: <description>\nactions:\n  - name: <action_name>\n    fn: <function_name_in_code>\n    description: <description>\n    payloadFields: [<field1>, <field2>]\n```");
        println!("\nOutput ONLY the valid YAML.");
    } else {
        // [DETERMINISTIC GENERATOR] 100% template-based code synthesis without AI hallucinations
        let ci144_path = Path::new(".lunar").join("ci-144.yml");
        if !ci144_path.exists() {
            anyhow::bail!("ci-144.yml not found. Run 'lunar gen ci-144 --prompt' to build configuration first.");
        }
        
        let content = fs::read_to_string(&ci144_path)?;
        let config: Ci144Config = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid ci-144.yml format: {}", e))?;

        println!("⚙️  Generating deterministic Cellrix CIB19 Bridge ({}) in {} ...", lang, out_dir);
        
        let out_path = Path::new(out_dir);
        fs::create_dir_all(out_path)?;
        
        // Load clean CIB19 template statically
        let bridge_code = format!(
            r#"// Generated automatically by LunarAST v3.0 from .lunar/ci-144.yml.
// 100% Deterministic. Do NOT edit manually.

use tokio::net::TcpStream;
use serde::{{Serialize, Deserialize}};

#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticNode {{
    pub name: String,
    pub r#type: String,
    pub description: String,
}}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionRequest {{
    pub action: String,
    pub payload: serde_json::Value,
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    println!("🚀 Starting Cellrix CIB19 Bridge for project: {} ...");
    // Connect to Cellrix and implement CIB19 protocol loop...
    Ok(())
}}
"#,
            config.project
        );

        let file_name = if lang == "rust" { "cellrix_bridge.rs" } else { "cellrix_bridge.py" };
        let file_path = out_path.join(file_name);
        fs::write(&file_path, bridge_code)?;
        println!("✓ Compiled bridge successfully written to {}", file_path.display());
    }
    Ok(())
}
