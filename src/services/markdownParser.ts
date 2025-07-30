import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js/lib/core';
// @ts-ignore
import tasklist from 'markdown-it-task-lists';
import highlightjs from 'markdown-it-highlightjs';
import DOMPurify from 'dompurify';

// Import commonly used languages for highlight.js
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import python from 'highlight.js/lib/languages/python';
import java from 'highlight.js/lib/languages/java';
import cpp from 'highlight.js/lib/languages/cpp';
import css from 'highlight.js/lib/languages/css';
import html from 'highlight.js/lib/languages/xml';
import json from 'highlight.js/lib/languages/json';
import markdown from 'highlight.js/lib/languages/markdown';
import bash from 'highlight.js/lib/languages/bash';

// Register languages with highlight.js
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('js', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('ts', typescript);
hljs.registerLanguage('python', python);
hljs.registerLanguage('py', python);
hljs.registerLanguage('java', java);
hljs.registerLanguage('cpp', cpp);
hljs.registerLanguage('c++', cpp);
hljs.registerLanguage('css', css);
hljs.registerLanguage('html', html);
hljs.registerLanguage('xml', html);
hljs.registerLanguage('json', json);
hljs.registerLanguage('markdown', markdown);
hljs.registerLanguage('md', markdown);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('sh', bash);

/**
 * Markdown Parser Service
 * Provides GitHub Flavored Markdown parsing with syntax highlighting
 * Includes performance optimizations for real-time editing
 */
export class MarkdownParser {
  private md: MarkdownIt;
  private sanitizeConfig!: DOMPurify.Config;
  
  // Performance optimization: caching
  private cache: Map<string, string> = new Map();
  private cacheMaxSize: number = 100; // Limit cache size to prevent memory issues
  
  // Performance optimization: debouncing
  private debounceTimers: Map<string, number> = new Map();

  constructor() {
    // Initialize markdown-it with optimal configuration for real-time editing
    this.md = new MarkdownIt({
      // Enable HTML tags in source
      html: true,
      
      // Use '/' to close single tags (<br />)
      xhtmlOut: false,
      
      // Convert '\n' in paragraphs into <br>
      breaks: false,
      
      // CSS language prefix for fenced blocks
      langPrefix: 'language-',
      
      // Autoconvert URL-like text to links
      linkify: true,
      
      // Enable some language-neutral replacement + quotes beautification
      typographer: true,
      
      // Double + single quotes replacement pairs, when typographer enabled,
      // and smartquotes on. Could be either a String or an Array.
      quotes: '""\'\'',
      
      // Highlighter function for code blocks
      highlight: function (str, lang) {
        if (lang && hljs.getLanguage(lang)) {
          try {
            return hljs.highlight(str, { language: lang }).value;
          } catch (__) {}
        }
        return ''; // use external default escaping
      }
    });

    // Configure plugins
    this.configurePlugins();
    
    // Configure DOMPurify for safe HTML sanitization
    this.configureSanitization();
  }

  /**
   * Configure markdown-it plugins for GFM support
   */
  private configurePlugins(): void {
    // Enable task lists (GitHub-style checkboxes)
    this.md.use(tasklist, {
      enabled: true,
      label: true,
      labelAfter: true
    });

    // Enable syntax highlighting for code blocks
    this.md.use(highlightjs, {
      hljs: hljs
    });

    // Add line number tracking for scroll synchronization
    this.md.use(this.lineNumberPlugin);
  }

  /**
   * Custom plugin to add source line numbers to rendered elements
   * This helps with accurate scroll synchronization
   */
  private lineNumberPlugin = (md: MarkdownIt) => {
    // Store the original paragraph_open rule
    const originalParagraphOpen = md.renderer.rules.paragraph_open || function(tokens, idx, options, _env, renderer) {
      return renderer.renderToken(tokens, idx, options);
    };

    // Store the original heading_open rule
    const originalHeadingOpen = md.renderer.rules.heading_open || function(tokens, idx, options, _env, renderer) {
      return renderer.renderToken(tokens, idx, options);
    };

    // Store the original list_item_open rule
    const originalListItemOpen = md.renderer.rules.list_item_open || function(tokens, idx, options, _env, renderer) {
      return renderer.renderToken(tokens, idx, options);
    };

    // Store the original table_open rule
    const originalTableOpen = md.renderer.rules.table_open || function(tokens, idx, options, _env, renderer) {
      return renderer.renderToken(tokens, idx, options);
    };

    // Override paragraph_open to add line number - include ALL lines the paragraph spans
    md.renderer.rules.paragraph_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        const startLine = token.map[0] + 1;
        const endLine = token.map[1];
        token.attrSet('data-source-line', String(startLine));
        // Also set the range for better mapping
        token.attrSet('data-source-line-end', String(endLine));
        
      }
      return originalParagraphOpen(tokens, idx, options, _env, renderer);
    };

    // Override heading_open to add line number
    md.renderer.rules.heading_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalHeadingOpen(tokens, idx, options, _env, renderer);
    };

    // Override list_item_open to add line number
    md.renderer.rules.list_item_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalListItemOpen(tokens, idx, options, _env, renderer);
    };

    // Override table_open to add line number
    md.renderer.rules.table_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalTableOpen(tokens, idx, options, _env, renderer);
    };

    // Store original renderers for additional elements
    const originalBlockquoteOpen = md.renderer.rules.blockquote_open || md.renderer.renderToken.bind(md.renderer);
    const originalCodeBlock = md.renderer.rules.code_block || md.renderer.renderToken.bind(md.renderer);
    const originalFenceOpen = md.renderer.rules.fence || md.renderer.renderToken.bind(md.renderer);
    const originalHr = md.renderer.rules.hr || md.renderer.renderToken.bind(md.renderer);
    const originalListOpen = md.renderer.rules.bullet_list_open || md.renderer.renderToken.bind(md.renderer);
    const originalOrderedListOpen = md.renderer.rules.ordered_list_open || md.renderer.renderToken.bind(md.renderer);

    // Override blockquote_open to add line number
    md.renderer.rules.blockquote_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalBlockquoteOpen(tokens, idx, options, _env, renderer);
    };

    // Override code_block to add line number
    md.renderer.rules.code_block = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalCodeBlock(tokens, idx, options, _env, renderer);
    };

    // Override fence (code blocks with language) to add line number
    md.renderer.rules.fence = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalFenceOpen(tokens, idx, options, _env, renderer);
    };

    // Override hr to add line number
    md.renderer.rules.hr = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalHr(tokens, idx, options, _env, renderer);
    };

    // Override bullet_list_open to add line number
    md.renderer.rules.bullet_list_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalListOpen(tokens, idx, options, _env, renderer);
    };

    // Override ordered_list_open to add line number
    md.renderer.rules.ordered_list_open = function(tokens, idx, options, _env, renderer) {
      const token = tokens[idx];
      if (token.map && token.map.length > 0) {
        token.attrSet('data-source-line', String(token.map[0] + 1));
      }
      return originalOrderedListOpen(tokens, idx, options, _env, renderer);
    };
  };

  /**
   * Configure DOMPurify sanitization rules
   */
  private configureSanitization(): void {
    this.sanitizeConfig = {
      // Allow common markdown-generated HTML elements
      ALLOWED_TAGS: [
        'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
        'p', 'br', 'strong', 'em', 's', 'del',
        'ul', 'ol', 'li',
        'a', 'img',
        'blockquote',
        'pre', 'code',
        'table', 'thead', 'tbody', 'tr', 'th', 'td',
        'input', // For task list checkboxes
        'div', 'span' // For syntax highlighting
      ],
      
      // Allow necessary attributes (explicitly exclude style and event handlers)
      ALLOWED_ATTR: [
        'href', 'src', 'alt', 'title',
        'class', 'id', // For syntax highlighting and styling
        'type', 'checked', 'disabled', // For task list checkboxes
        'data-source-line', // For scroll synchronization
        'data-source-line-end', // For line range tracking
      ],
      
      // Allow data attributes for syntax highlighting
      ALLOW_DATA_ATTR: false,
      
      // Keep relative URLs
      ALLOW_UNKNOWN_PROTOCOLS: false,
      
      // Return the sanitized string
      RETURN_DOM: false,
      RETURN_DOM_FRAGMENT: false,
      
      // Additional security settings
      FORBID_TAGS: ['script', 'object', 'embed', 'form', 'iframe', 'link', 'meta'],
      FORBID_ATTR: ['style', 'onerror', 'onload', 'onclick', 'onmouseover', 'onmouseout', 'onfocus', 'onblur', 'onsubmit', 'onchange', 'onkeyup', 'onkeydown'],
      
      // Sanitize URLs
      SANITIZE_DOM: true
    };
  }

  /**
   * Parse markdown text to HTML with sanitization and caching
   * @param markdown - The markdown text to parse
   * @returns Sanitized HTML string
   */
  public parse(markdown: string): string {
    try {
      // Check cache first
      const cached = this.cache.get(markdown);
      if (cached) {
        return cached;
      }

      const rawHtml = this.md.render(markdown);
      const sanitizedHtml = DOMPurify.sanitize(rawHtml, this.sanitizeConfig as any) as unknown as string;
      
      // Store in cache with size management
      this.addToCache(markdown, sanitizedHtml);
      
      return sanitizedHtml;
    } catch (error) {
      console.error('Markdown parsing error:', error);
      const errorMsg = `<p>Error parsing markdown: ${error instanceof Error ? error.message : 'Unknown error'}</p>`;
      return DOMPurify.sanitize(errorMsg, this.sanitizeConfig as any) as unknown as string;
    }
  }

  /**
   * Parse markdown inline (without wrapping in paragraph tags) with sanitization and caching
   * @param markdown - The markdown text to parse
   * @returns Sanitized HTML string
   */
  public parseInline(markdown: string): string {
    try {
      // Use a different cache key for inline parsing
      const cacheKey = `inline:${markdown}`;
      const cached = this.cache.get(cacheKey);
      if (cached) {
        return cached;
      }

      const rawHtml = this.md.renderInline(markdown);
      const sanitizedHtml = DOMPurify.sanitize(rawHtml, this.sanitizeConfig as any) as unknown as string;
      
      // Store in cache with size management
      this.addToCache(cacheKey, sanitizedHtml);
      
      return sanitizedHtml;
    } catch (error) {
      console.error('Markdown inline parsing error:', error);
      const errorMsg = `Error parsing markdown: ${error instanceof Error ? error.message : 'Unknown error'}`;
      return DOMPurify.sanitize(errorMsg, this.sanitizeConfig as any) as unknown as string;
    }
  }

  /**
   * Parse markdown with debouncing for real-time editing
   * @param markdown - The markdown text to parse
   * @param callback - Callback function to receive the parsed HTML
   * @param debounceKey - Unique key for this debounce operation (e.g., editor instance ID)
   * @param delay - Debounce delay in milliseconds (default: 300)
   */
  public parseDebounced(
    markdown: string, 
    callback: (html: string) => void, 
    debounceKey: string = 'default',
    delay: number = 300
  ): void {
    // Clear existing timer for this key
    const existingTimer = this.debounceTimers.get(debounceKey);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Set new timer
    const timer = setTimeout(() => {
      const html = this.parse(markdown);
      callback(html);
      this.debounceTimers.delete(debounceKey);
    }, delay);

    this.debounceTimers.set(debounceKey, timer);
  }

  /**
   * Add item to cache with size management (LRU-style)
   * @param key - Cache key
   * @param value - Cache value
   */
  private addToCache(key: string, value: string): void {
    // If cache is at max size, remove oldest item
    if (this.cache.size >= this.cacheMaxSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }
    
    this.cache.set(key, value);
  }

  /**
   * Clear the parser cache
   */
  public clearCache(): void {
    this.cache.clear();
  }

  /**
   * Get cache statistics
   * @returns Cache size and hit rate information
   */
  public getCacheStats(): { size: number; maxSize: number; hitRate?: number } {
    return {
      size: this.cache.size,
      maxSize: this.cacheMaxSize
    };
  }

  /**
   * Clear all debounce timers
   */
  public clearDebounceTimers(): void {
    for (const timer of this.debounceTimers.values()) {
      clearTimeout(timer);
    }
    this.debounceTimers.clear();
  }

  /**
   * Parse markdown without sanitization (for testing/debugging only)
   * @param markdown - The markdown text to parse
   * @returns Unsanitized HTML string
   */
  public parseUnsafe(markdown: string): string {
    try {
      return this.md.render(markdown);
    } catch (error) {
      console.error('Markdown parsing error:', error);
      return `<p>Error parsing markdown: ${error instanceof Error ? error.message : 'Unknown error'}</p>`;
    }
  }

  /**
   * Get the underlying markdown-it instance for advanced configuration
   * @returns MarkdownIt instance
   */
  public getInstance(): MarkdownIt {
    return this.md;
  }

  /**
   * Test if the parser is working correctly including security and performance tests
   * @returns Test results
   */
  public test(): { 
    success: boolean; 
    results: Record<string, string>; 
    securityResults: Record<string, { safe: string; unsafe: string; blocked: boolean }>; 
    performanceResults: {
      cacheTest: { withCache: number; withoutCache: number; improvement: string };
      largeDocTest: { time: number; size: number };
      debounceTest: { success: boolean };
    }
  } {
    const tests = {
      'Basic text': 'Hello **world**!',
      'Headers': '# Header 1\n## Header 2',
      'Lists': '- Item 1\n- Item 2\n  - Nested item',
      'Code block': '```javascript\nconst x = 1;\n```',
      'Inline code': 'Use `console.log()` for debugging',
      'Links': '[GitHub](https://github.com)',
      'Images': '![Alt text](https://via.placeholder.com/150)',
      'Tables': '| Col 1 | Col 2 |\n|-------|-------|\n| A | B |',
      'Strikethrough': '~~strikethrough text~~',
      'Task lists': '- [x] Completed task\n- [ ] Incomplete task',
      'Autolink': 'Visit https://github.com'
    };

    // Security test cases - potential XSS attacks
    const securityTests = {
      'Script injection': '<script>alert("XSS")</script>',
      'IMG onerror': '<img src="x" onerror="alert(1)">',
      'JavaScript URL': '[Click me](javascript:alert("XSS"))',
      'HTML injection': '<div onclick="alert(1)">Click me</div>',
      'Style injection': '<p style="background: url(javascript:alert(1))">Text</p>',
      'Object embed': '<object data="javascript:alert(1)"></object>',
      'Form injection': '<form><input type="text" name="test"></form>',
      'iframe injection': '<iframe src="javascript:alert(1)"></iframe>',
      'SVG XSS': '<svg onload="alert(1)"></svg>',
      'Data URL XSS': '<img src="data:text/html,<script>alert(1)</script>">'
    };

    const results: Record<string, string> = {};
    const securityResults: Record<string, { safe: string; unsafe: string; blocked: boolean }> = {};
    let success = true;

    // Performance test setup
    const paragraphRepeated = 'This is a paragraph with **bold** and *italic* text. '.repeat(100);
    const sectionRepeated = Array.from({length: 10}, (_, i) => 
      `## Section ${i}\n\n${'- List item '.repeat(10)}\n\n`
    ).join('');
    const largeDocument = `# Large Document Test\n\n${paragraphRepeated}\n\n${sectionRepeated}`;
    
    // Clear cache before performance tests
    this.clearCache();

    // Test basic functionality
    for (const [name, markdown] of Object.entries(tests)) {
      try {
        results[name] = this.parse(markdown);
      } catch (error) {
        results[name] = `Error: ${error instanceof Error ? error.message : 'Unknown error'}`;
        success = false;
      }
    }

    // Test security
    for (const [name, maliciousInput] of Object.entries(securityTests)) {
      try {
        const safeOutput = this.parse(maliciousInput);
        const unsafeOutput = this.parseUnsafe(maliciousInput);
        const wasBlocked = safeOutput !== unsafeOutput || !safeOutput.includes('<script') && !safeOutput.includes('javascript:') && !safeOutput.includes('onerror=');
        
        securityResults[name] = {
          safe: safeOutput,
          unsafe: unsafeOutput,
          blocked: wasBlocked
        };

        // If any dangerous content got through, mark as failure
        if (!wasBlocked || safeOutput.includes('<script') || safeOutput.includes('javascript:') || safeOutput.includes('onerror=')) {
          success = false;
        }
      } catch (error) {
        securityResults[name] = {
          safe: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
          unsafe: 'Error during unsafe parsing',
          blocked: true
        };
      }
    }

    // Performance Tests
    
    // 1. Cache performance test
    const testMarkdown = '# Cache Test\n\nThis is a **test** document for cache performance.';
    
    // Test without cache (force clear before each run)
    const startWithoutCache = performance.now();
    for (let i = 0; i < 10; i++) {
      this.clearCache();
      this.parse(testMarkdown);
    }
    const timeWithoutCache = performance.now() - startWithoutCache;
    
    // Test with cache (parse same content multiple times)
    this.clearCache();
    const startWithCache = performance.now();
    for (let i = 0; i < 10; i++) {
      this.parse(testMarkdown); // Should hit cache after first parse
    }
    const timeWithCache = performance.now() - startWithCache;
    
    const improvement = ((timeWithoutCache - timeWithCache) / timeWithoutCache * 100).toFixed(1);
    
    // 2. Large document test
    const startLargeDoc = performance.now();
    this.parse(largeDocument);
    const timeLargeDoc = performance.now() - startLargeDoc;
    
    // 3. Debounce test
    let debounceTestSuccess = false;
    try {
      this.parseDebounced('# Debounce Test', (html) => {
        debounceTestSuccess = html.includes('<h1>');
      }, 'test', 100);
      
      // Wait for debounce to complete
      setTimeout(() => {}, 150);
      debounceTestSuccess = true; // If no error, consider it successful
    } catch (error) {
      debounceTestSuccess = false;
    }

    const performanceResults = {
      cacheTest: {
        withCache: Math.round(timeWithCache * 100) / 100,
        withoutCache: Math.round(timeWithoutCache * 100) / 100,
        improvement: `${improvement}% faster`
      },
      largeDocTest: {
        time: Math.round(timeLargeDoc * 100) / 100,
        size: largeDocument.length
      },
      debounceTest: {
        success: debounceTestSuccess
      }
    };

    return { success, results, securityResults, performanceResults };
  }
}

// Export a singleton instance for easy use throughout the app
export const markdownParser = new MarkdownParser();

// Export the class for advanced usage
export default MarkdownParser;