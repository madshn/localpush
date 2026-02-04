import { TransparencyPreview } from "./TransparencyPreview";

/**
 * Example usage of TransparencyPreview component
 *
 * This demonstrates the radical transparency principle:
 * Users see REAL data before enabling a source.
 */

// Example 1: Claude Code Statistics with positive trend
const claudeCodePreview = {
  title: "Claude Code Statistics",
  summary: "161,472 tokens today +15% ↑",
  lastUpdated: new Date().toISOString(),
  fields: [
    { label: "Total tokens", value: "161,472", sensitive: false },
    { label: "Sessions", value: "23", sensitive: false },
    { label: "Files modified", value: "47", sensitive: false },
    { label: "API Key", value: "sk-ant-api03-abc123xyz789", sensitive: true },
    { label: "Project path", value: "/Users/madsnissen/dev/localpush", sensitive: false },
  ],
};

// Example 2: GitHub Activity with negative trend
const githubPreview = {
  title: "GitHub Activity",
  summary: "12 commits today -10% ↓",
  lastUpdated: new Date(Date.now() - 3600000).toISOString(), // 1 hour ago
  fields: [
    { label: "Commits", value: "12", sensitive: false },
    { label: "PRs opened", value: "2", sensitive: false },
    { label: "Issues closed", value: "5", sensitive: false },
    { label: "Access token", value: "ghp_1234567890abcdefghijklmnop", sensitive: true },
    { label: "Username", value: "madsnissen", sensitive: false },
  ],
};

// Example 3: No trend in summary
const simplePreview = {
  title: "System Metrics",
  summary: "CPU: 45%, Memory: 8.2GB",
  lastUpdated: null,
  fields: [
    { label: "CPU usage", value: "45%", sensitive: false },
    { label: "Memory", value: "8.2GB", sensitive: false },
    { label: "Disk", value: "512GB free", sensitive: false },
  ],
};

export function TransparencyPreviewExamples() {
  const handleEnable = () => {
    console.log("Source enabled");
  };

  const handleRefresh = () => {
    console.log("Refreshing preview...");
  };

  return (
    <div style={{ padding: "20px", display: "flex", flexDirection: "column", gap: "20px" }}>
      <h1>Transparency Preview Examples</h1>

      <section>
        <h2>Example 1: With Positive Trend</h2>
        <TransparencyPreview
          sourceId="claude-code"
          preview={claudeCodePreview}
          onEnable={handleEnable}
          onRefresh={handleRefresh}
        />
      </section>

      <section>
        <h2>Example 2: With Negative Trend</h2>
        <TransparencyPreview
          sourceId="github"
          preview={githubPreview}
          onEnable={handleEnable}
          onRefresh={handleRefresh}
        />
      </section>

      <section>
        <h2>Example 3: No Trend</h2>
        <TransparencyPreview
          sourceId="system"
          preview={simplePreview}
          onEnable={handleEnable}
          onRefresh={handleRefresh}
        />
      </section>

      <section>
        <h2>Example 4: Loading State</h2>
        <TransparencyPreview
          sourceId="loading"
          preview={simplePreview}
          onEnable={handleEnable}
          onRefresh={handleRefresh}
          isLoading={true}
        />
      </section>
    </div>
  );
}
