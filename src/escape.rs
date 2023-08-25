pub fn escape_js_str(str: &str) -> String {
    str.replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\"", "\\\"")
        .replace("\'", "\\\'")
        .replace("\\", "\\\\")
}
