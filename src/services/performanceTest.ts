import { markdownParser } from './markdownParser';

/**
 * Performance test suite for the optimized markdown parser
 */
export class PerformanceTest {
  /**
   * Run comprehensive performance tests
   */
  public static runTests(): void {
    console.log('🚀 Running Markdown Parser Performance Tests...\n');

    // Test 1: Cache Performance
    this.testCachePerformance();

    // Test 2: Large Document Performance
    this.testLargeDocumentPerformance();

    // Test 3: Debouncing Functionality
    this.testDebouncingFunctionality();

    // Test 4: Memory Usage
    this.testMemoryUsage();

    console.log('✅ Performance tests completed!\n');
  }

  /**
   * Test caching performance improvements
   */
  private static testCachePerformance(): void {
    console.log('📊 Testing Cache Performance...');

    const testDocument = `# Performance Test Document

This is a **comprehensive** test document that includes:

- Lists with multiple items
- Code blocks with syntax highlighting
- Tables and other complex elements

\`\`\`javascript
function testFunction() {
  console.log("This is a test");
  return true;
}
\`\`\`

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Data 1   | Data 2   | Data 3   |
| More     | Test     | Data     |

## Performance Characteristics

The parser should handle this content efficiently with caching.
`;

    // Clear cache first
    markdownParser.clearCache();

    // Test without cache (multiple parses)
    const startWithoutCache = performance.now();
    for (let i = 0; i < 50; i++) {
      markdownParser.clearCache();
      markdownParser.parse(testDocument);
    }
    const timeWithoutCache = performance.now() - startWithoutCache;

    // Test with cache (same content multiple times)
    markdownParser.clearCache();
    const startWithCache = performance.now();
    for (let i = 0; i < 50; i++) {
      markdownParser.parse(testDocument); // Should hit cache after first parse
    }
    const timeWithCache = performance.now() - startWithCache;

    const improvement = ((timeWithoutCache - timeWithCache) / timeWithoutCache * 100);

    console.log(`   Without cache: ${timeWithoutCache.toFixed(2)}ms`);
    console.log(`   With cache: ${timeWithCache.toFixed(2)}ms`);
    console.log(`   Improvement: ${improvement.toFixed(1)}% faster\n`);

    const cacheStats = markdownParser.getCacheStats();
    console.log(`   Cache stats: ${cacheStats.size}/${cacheStats.maxSize} items\n`);
  }

  /**
   * Test performance with large documents
   */
  private static testLargeDocumentPerformance(): void {
    console.log('📈 Testing Large Document Performance...');

    // Generate a large markdown document
    const largeDocument = this.generateLargeDocument();
    
    console.log(`   Document size: ${largeDocument.length} characters`);

    // Test parsing time
    const start = performance.now();
    const result = markdownParser.parse(largeDocument);
    const parseTime = performance.now() - start;

    console.log(`   Parse time: ${parseTime.toFixed(2)}ms`);
    console.log(`   Output size: ${result.length} characters`);
    console.log(`   Performance: ${(largeDocument.length / parseTime).toFixed(0)} chars/ms\n`);

    // Test with cache
    const cachedStart = performance.now();
    markdownParser.parse(largeDocument); // Should hit cache
    const cachedTime = performance.now() - cachedStart;

    console.log(`   Cached parse time: ${cachedTime.toFixed(2)}ms`);
    console.log(`   Cache speedup: ${(parseTime / cachedTime).toFixed(1)}x faster\n`);
  }

  /**
   * Test debouncing functionality
   */
  private static async testDebouncingFunctionality(): Promise<void> {
    console.log('⏱️  Testing Debouncing Functionality...');

    return new Promise<void>((resolve) => {
      let callCount = 0;
      const testText = '# Debounce Test\n\nThis tests debouncing.';

      // Simulate rapid typing (multiple calls)
      for (let i = 0; i < 10; i++) {
        markdownParser.parseDebounced(
          testText + i,
          (_html) => {
            callCount++;
            console.log(`   Debounced callback called (${callCount} times)`);
          },
          'debounce-test',
          100
        );
      }

      // Check after debounce period
      setTimeout(() => {
        console.log(`   Expected: 1 call, Actual: ${callCount} calls`);
        console.log(`   Debouncing ${callCount === 1 ? '✅ PASSED' : '❌ FAILED'}\n`);
        resolve();
      }, 200);
    });
  }

  /**
   * Test memory usage and cache management
   */
  private static testMemoryUsage(): void {
    console.log('💾 Testing Memory Usage...');

    // Fill cache to capacity
    for (let i = 0; i < 150; i++) {
      markdownParser.parse(`# Document ${i}\n\nContent for document ${i}`);
    }

    const stats = markdownParser.getCacheStats();
    console.log(`   Cache size after 150 parses: ${stats.size}/${stats.maxSize}`);
    console.log(`   Memory management: ${stats.size <= stats.maxSize ? '✅ PASSED' : '❌ FAILED'}\n`);

    // Clear cache
    markdownParser.clearCache();
    const clearedStats = markdownParser.getCacheStats();
    console.log(`   Cache size after clear: ${clearedStats.size}`);
    console.log(`   Cache clearing: ${clearedStats.size === 0 ? '✅ PASSED' : '❌ FAILED'}\n`);
  }

  /**
   * Generate a large test document
   */
  private static generateLargeDocument(): string {
    const sections = [];
    
    for (let i = 1; i <= 20; i++) {
      sections.push(`## Section ${i}

This is section ${i} with **bold text**, *italic text*, and ~~strikethrough~~.

### Subsection ${i}.1

Here's a list:
- Item 1 for section ${i}
- Item 2 for section ${i}
- Item 3 for section ${i}
  - Nested item A
  - Nested item B

### Code Example ${i}

\`\`\`javascript
function section${i}Function() {
  console.log("This is section ${i}");
  const data = {
    section: ${i},
    active: true,
    items: [1, 2, 3, 4, 5]
  };
  return data;
}
\`\`\`

### Table ${i}

| Column A | Column B | Column C |
|----------|----------|----------|
| Data ${i}1 | Data ${i}2 | Data ${i}3 |
| Value ${i}1 | Value ${i}2 | Value ${i}3 |

`);
    }

    return `# Large Document Performance Test

This document contains multiple sections to test parser performance.

${sections.join('\n')}

## Conclusion

This document has been generated to test the performance of the markdown parser with a substantial amount of content including various markdown features.
`;
  }
}

// Export the test runner
export default PerformanceTest;