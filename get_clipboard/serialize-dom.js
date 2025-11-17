/**
 * Generates a compact ASCII tree representation of the DOM
 * @returns {string} ASCII tree representation
 */

const IGNORE_SELECTORS = [
  'link[rel="icon"]',
  'link[rel="shortcut icon"]',
  'link[rel="apple-touch-icon"]',
  'link[rel="apple-touch-icon-precomposed"]',
  '[aria-hidden="true"]',
  'link[rel="manifest"]',
  'link[rel*="preload"]',
  element => {
    const tagName = element.tagName?.toLowerCase();
    
    if (tagName === 'script') {
      const src = element.src || element.getAttribute('src');
      if (src && /[-.][0-9a-f-]{8,}\.(?:js|css)$/i.test(src)) {
        return true;
      }
    }
    
    if (tagName === 'link') {
      const href = element.href || element.getAttribute('href');
      if (href && /[-.][0-9a-f-]{8,}\.(?:js|css)$/i.test(href)) {
        return true;
      }
    }
    
    return false;
  }
];

function serializeDOM() {
  const root = document.documentElement;
  const classUsage = new Map();
  const elementClassMap = new Map();

  function analyzeClasses(element) {
    if (element.nodeType !== Node.ELEMENT_NODE) return;

    const classes = Array.from(element.classList || []);
    elementClassMap.set(element, classes);

    classes.forEach(className => {
      if (!classUsage.has(className)) {
        classUsage.set(className, new Set());
      }
      classUsage.get(className).add(element);
    });

    for (const child of element.children) {
      analyzeClasses(child);
    }
  }

  function isRandomClass(className) {
    if (className.length < 2) return false;
    
    const hasManyNumbers = (className.match(/\d/g) || []).length / className.length > 0.5;
    const hasRepeatedChars = /(.)\1{3,}/.test(className);
    const hasHexPattern = /^[0-9a-f]{8,}/i.test(className);
    const randomPatterns = /^(css-|class-|sc-|st-|_|__)/i.test(className);
    
    return hasManyNumbers || hasRepeatedChars || hasHexPattern || randomPatterns;
  }

  function getSiblingSharedClasses(element) {
    if (!element.parentElement) return new Set();
    
    const siblings = Array.from(element.parentElement.children);
    if (siblings.length < 2) return new Set();

    const siblingClasses = siblings.map(s => Array.from(s.classList || []));
    const sharedClasses = new Set(siblingClasses[0]);

    for (let i = 1; i < siblingClasses.length; i++) {
      const currentClasses = new Set(siblingClasses[i]);
      sharedClasses.forEach(cls => {
        if (!currentClasses.has(cls)) {
          sharedClasses.delete(cls);
        }
      });
    }

    return sharedClasses;
  }

  function getMeaningfulClasses(element, classes) {
    if (!classes || classes.length === 0) return [];

    const usageCount = classUsage.size;
    const elementClassUsage = new Map();
    
    classes.forEach(cls => {
      const usage = classUsage.get(cls)?.size || 0;
      elementClassUsage.set(cls, usage);
    });

    const siblingShared = getSiblingSharedClasses(element);
    
    const meaningful = [];
    let keptOneSharedClass = false;

    const sortedClasses = Array.from(classes).sort((a, b) => {
      const usageA = elementClassUsage.get(a) || 0;
      const usageB = elementClassUsage.get(b) || 0;
      if (usageA === usageB) return 0;
      return usageA < usageB ? -1 : 1;
    });

    // First, collect specific classes (current behavior)
    for (const cls of sortedClasses) {
      const usage = elementClassUsage.get(cls) || 0;
      const isSharedWithSiblings = siblingShared.has(cls);
      
      if (isRandomClass(cls)) continue;

      const usageThreshold = Math.max(5, Math.floor(usageCount * 0.05));
      if (usage > usageThreshold) {
        if (usage > 1 && usage <= 3) {
          meaningful.push(cls);
        }
        continue;
      }

      if (isSharedWithSiblings && keptOneSharedClass) {
        continue;
      }
      
      if (isSharedWithSiblings) {
        keptOneSharedClass = true;
      }

      meaningful.push(cls);
      
      if (meaningful.length >= 3) break;
    }

    // Now add the two most general classes (highest usage across all elements)
    // Sort all classes by usage (most general first)
    const allClassesByUsage = Array.from(classUsage.entries())
      .map(([cls, elements]) => ({ cls, usage: elements.size }))
      .filter(({ cls }) => !isRandomClass(cls))
      .sort((a, b) => b.usage - a.usage);

    // Find the two most general classes that this element has
    const generalClasses = [];
    for (const { cls } of allClassesByUsage) {
      if (classes.includes(cls) && !meaningful.includes(cls)) {
        generalClasses.push(cls);
        if (generalClasses.length >= 2) break;
      }
    }

    // Add general classes to meaningful (they'll be sorted later)
    meaningful.push(...generalClasses);

    return meaningful;
  }

  function getTextPreview(element) {
    let directText = '';
    for (const child of element.childNodes) {
      if (child.nodeType === Node.TEXT_NODE) {
        directText += child.textContent;
      }
    }
    
    const trimmed = directText.trim();
    if (!trimmed) return '';
    
    const maxLength = 50;
    const preview = trimmed.length > maxLength 
      ? trimmed.substring(0, maxLength) + '...'
      : trimmed;
    
    return preview.replace(/\s+/g, ' ');
  }

  function countRecursiveChildren(element) {
    let count = 0;
    const children = Array.from(element.children).filter(child =>
      !shouldSkipElement(child)
    );
    
    count += children.length;
    children.forEach(child => {
      count += countRecursiveChildren(child);
    });
    
    return count;
  }

  function getInnerTextInfo(element) {
    const innerText = element.innerText || element.textContent || '';
    const trimmed = innerText.trim();
    const length = trimmed.length;
    
    if (length === 0) return null;
    
    const preview = length > 100 
      ? trimmed.substring(0, 100) + '...'
      : trimmed;
    
    return {
      length,
      preview: preview.replace(/\s+/g, ' ')
    };
  }

  function shouldSkipElement(element) {
    if (element.nodeType !== Node.ELEMENT_NODE) return false;

    const tagName = element.tagName.toLowerCase();
    
    if (['meta', 'noscript'].includes(tagName)) return true;
    
    for (const selector of IGNORE_SELECTORS) {
      if (typeof selector === 'function') {
        if (selector(element)) return true;
      } else {
        if (element.matches(selector)) return true;
      }
    }
    
    return false;
  }

  function findCommonSelector(elements) {
    if (elements.length === 0) return '';
    if (elements.length === 1) return generateSelector(elements[0]);

    const tagName = elements[0].tagName.toLowerCase();
    
    // Check if all elements have the same tag
    if (!elements.every(el => el.tagName.toLowerCase() === tagName)) {
      return tagName;
    }

    // Try to find common ID
    const ids = elements.map(el => el.id).filter(Boolean);
    if (ids.length === elements.length && new Set(ids).size === 1) {
      return `${tagName}#${ids[0]}`;
    }

    // Try to find common classes
    const allClasses = elements.map(el => {
      const classes = elementClassMap.get(el) || [];
      return new Set(classes);
    });

    if (allClasses.length > 0) {
      const commonClasses = Array.from(allClasses[0]).filter(cls =>
        allClasses.every(classSet => classSet.has(cls))
      );

      if (commonClasses.length > 0) {
        const meaningfulCommon = commonClasses.filter(cls => !isRandomClass(cls));
        if (meaningfulCommon.length > 0) {
          return `${tagName}${meaningfulCommon.map(c => `.${c}`).join('')}`;
        }
      }
    }

    // Try to find common attributes
    const firstAttrs = new Map();
    Array.from(elements[0].attributes || []).forEach(attr => {
      if (!attr.name.startsWith('data-') && attr.name !== 'class' && attr.name !== 'id') {
        firstAttrs.set(attr.name, attr.value);
      }
    });

    const commonAttrs = [];
    for (const [name, value] of firstAttrs) {
      if (elements.every(el => {
        const attr = el.getAttribute(name);
        return attr === value;
      })) {
        commonAttrs.push(`[${name}="${value}"]`);
      }
    }

    if (commonAttrs.length > 0) {
      return `${tagName}${commonAttrs.join('')}`;
    }

    // Fall back to just tag name
    return tagName;
  }

  function generateSelector(element) {
    if (element.nodeType !== Node.ELEMENT_NODE) {
      return element.nodeName.toLowerCase();
    }

    const tagName = element.tagName.toLowerCase();
    const parts = [tagName];
    
    if (element.id) {
      parts.push(`#${element.id}`);
    }

    const classes = elementClassMap.get(element) || [];
    const meaningfulClasses = getMeaningfulClasses(element, classes);
    if (meaningfulClasses.length > 0) {
      const classString = meaningfulClasses.map(c => `.${c}`).join('');
      parts.push(classString);
    }

    const ariaAttrs = Array.from(element.attributes || []).filter(attr => 
      attr.name.startsWith('aria-') || attr.name === 'role'
    );
    if (ariaAttrs.length > 0) {
      const ariaString = ariaAttrs.map(attr => 
        `[${attr.name}="${attr.value}"]`
      ).join('');
      parts.push(ariaString);
    }

    const dataAttrs = Array.from(element.attributes || []).filter(attr => 
      attr.name.startsWith('data-')
    );
    if (dataAttrs.length > 0) {
      const dataString = dataAttrs.map(attr => {
        const value = attr.value.length > 100 
          ? attr.value.substring(0, 100) + '...' 
          : attr.value;
        return `[${attr.name}="${value}"]`;
      }).join('');
      parts.push(dataString);
    }

    // Media elements: src, alt, title
    if (tagName === 'img' || tagName === 'video' || tagName === 'audio' || tagName === 'iframe') {
      if (element.src) {
        parts.push(`[src="${element.src}"]`);
      }
      if (element.alt) {
        parts.push(`[alt="${element.alt}"]`);
      }
    }

    // Object elements use data attribute instead of src
    if (tagName === 'object') {
      if (element.data) {
        parts.push(`[data="${element.data}"]`);
      }
      if (element.alt) {
        parts.push(`[alt="${element.alt}"]`);
      }
    }

    // Iframe specific attributes
    if (tagName === 'iframe') {
      if (element.hasAttribute('sandbox')) {
        const sandbox = element.getAttribute('sandbox');
        if (sandbox) {
          parts.push(`[sandbox="${sandbox}"]`);
        } else {
          parts.push('[sandbox]');
        }
      }
      if (element.allow) {
        parts.push(`[allow="${element.allow}"]`);
      }
      if (element.loading) {
        parts.push(`[loading="${element.loading}"]`);
      }
    }

    // Title attribute for all elements
    if (element.title) {
      parts.push(`[title="${element.title}"]`);
    }

    // Video and audio specific attributes
    if (tagName === 'video' || tagName === 'audio') {
      if (element.hasAttribute('controls')) {
        parts.push('[controls]');
      }
      if (element.hasAttribute('muted')) {
        parts.push('[muted]');
      }
      if (element.hasAttribute('autoplay')) {
        parts.push('[autoplay]');
      }
      if (element.hasAttribute('loop')) {
        parts.push('[loop]');
      }
      if (element.hasAttribute('preload')) {
        parts.push(`[preload="${element.getAttribute('preload')}"]`);
      }
      // Video-specific: poster attribute
      if (tagName === 'video' && element.poster) {
        parts.push(`[poster="${element.poster}"]`);
      }
      // Runtime state (may not always be available)
      if (typeof element.paused === 'boolean' && !element.paused) {
        parts.push('[playing]');
      }
      if (typeof element.ended === 'boolean' && element.ended) {
        parts.push('[ended]');
      }
    }

    // Input elements
    if (tagName === 'input') {
      const inputType = element.type || 'text';
      parts.push(`[type="${inputType}"]`);
      
      if (element.value && inputType !== 'password') {
        const value = element.value.length > 50 
          ? element.value.substring(0, 50) + '...' 
          : element.value;
        parts.push(`[value="${value}"]`);
      }
      
      if (element.placeholder) {
        parts.push(`[placeholder="${element.placeholder}"]`);
      }
      
      if (element.checked !== undefined) {
        parts.push(`[checked="${element.checked}"]`);
      }
      
      if (element.disabled) {
        parts.push('[disabled]');
      }
      
      if (element.readOnly) {
        parts.push('[readonly]');
      }
      
      if (element.required) {
        parts.push('[required]');
      }
      
      if (element.min !== undefined && element.min !== '') {
        parts.push(`[min="${element.min}"]`);
      }
      
      if (element.max !== undefined && element.max !== '') {
        parts.push(`[max="${element.max}"]`);
      }
      
      if (element.step !== undefined && element.step !== '') {
        parts.push(`[step="${element.step}"]`);
      }
      
      if (element.maxLength && element.maxLength > 0) {
        parts.push(`[maxlength="${element.maxLength}"]`);
      }
      
      if (element.pattern) {
        parts.push(`[pattern="${element.pattern}"]`);
      }
      
      if (inputType === 'file' && element.accept) {
        parts.push(`[accept="${element.accept}"]`);
      }
    }

    // Select elements
    if (tagName === 'select') {
      if (element.multiple) {
        parts.push('[multiple]');
      }
      
      if (element.size && element.size > 1) {
        parts.push(`[size="${element.size}"]`);
      }
      
      if (element.disabled) {
        parts.push('[disabled]');
      }
      
      if (element.required) {
        parts.push('[required]');
      }
      
      // Show selected option value
      if (element.selectedIndex >= 0 && element.options[element.selectedIndex]) {
        const selectedOption = element.options[element.selectedIndex];
        const selectedValue = selectedOption.value || selectedOption.text;
        if (selectedValue) {
          const value = selectedValue.length > 50 
            ? selectedValue.substring(0, 50) + '...' 
            : selectedValue;
          parts.push(`[selected="${value}"]`);
        }
      }
    }

    // Option elements
    if (tagName === 'option') {
      if (element.selected) {
        parts.push('[selected]');
      }
      
      if (element.value) {
        const value = element.value.length > 50 
          ? element.value.substring(0, 50) + '...' 
          : element.value;
        parts.push(`[value="${value}"]`);
      }
    }

    // Textarea elements
    if (tagName === 'textarea') {
      if (element.placeholder) {
        parts.push(`[placeholder="${element.placeholder}"]`);
      }
      
      if (element.value) {
        const value = element.value.length > 50 
          ? element.value.substring(0, 50) + '...' 
          : element.value;
        parts.push(`[value="${value}"]`);
      }
      
      if (element.rows && element.rows > 1) {
        parts.push(`[rows="${element.rows}"]`);
      }
      
      if (element.cols && element.cols > 1) {
        parts.push(`[cols="${element.cols}"]`);
      }
      
      if (element.disabled) {
        parts.push('[disabled]');
      }
      
      if (element.readOnly) {
        parts.push('[readonly]');
      }
      
      if (element.required) {
        parts.push('[required]');
      }
      
      if (element.maxLength && element.maxLength > 0) {
        parts.push(`[maxlength="${element.maxLength}"]`);
      }
    }

    // Link elements
    if (tagName === 'link' && element.href) {
      parts.push(`[href="${element.href}"]`);
    }

    // Script elements
    if (tagName === 'script' && element.src) {
      parts.push(`[src="${element.src}"]`);
    }

    return parts.join('');
  }

  function serializeNode(node, indent = '', parentIsSVG = false, parentChain = [], level = 0) {
    const lines = [];

    if (node.nodeType !== Node.ELEMENT_NODE) return lines;

    if (shouldSkipElement(node)) return lines;

    const tagName = node.tagName.toLowerCase();
    const isSVG = tagName === 'svg' || parentIsSVG;

    // Check if element has distinguishing features
    const hasDistinguishingFeatures = !!(node.id || 
      (elementClassMap.get(node) || []).length > 0 ||
      Array.from(node.attributes || []).some(attr => 
        attr.name.startsWith('aria-') || attr.name === 'role' || 
        attr.name.startsWith('data-') || attr.name === 'src' || 
        attr.name === 'href' || attr.name === 'alt' || attr.name === 'title'
      )
    );

    const children = Array.from(node.children).filter(child =>
      !shouldSkipElement(child)
    );

    // Check if we should collapse a chain of same-tag elements
    let shouldCollapse = false;
    let collapseCount = 0;
    
    // Helper to check if element has distinguishing features
    const hasDistinguishing = (el) => {
      return !!(el.id || 
        (elementClassMap.get(el) || []).length > 0 ||
        Array.from(el.attributes || []).some(attr => 
          attr.name.startsWith('aria-') || attr.name === 'role' || 
          attr.name.startsWith('data-') || attr.name === 'src' || 
          attr.name === 'href' || attr.name === 'alt' || attr.name === 'title'
        )
      );
    };

    // Only collapse if current node doesn't have distinguishing features
    // and its first child continues the same-tag chain
    if (children.length > 0 && !hasDistinguishingFeatures) {
      const firstChild = children[0];
      const firstChildTag = firstChild.tagName.toLowerCase();
      const firstChildHasDistinguishing = hasDistinguishing(firstChild);

      if (firstChildTag === tagName && !firstChildHasDistinguishing) {
        // Count the chain starting from current node
        collapseCount = 1; // Start with current node
        
        // Continue counting down the chain
        let current = node;
        while (current && current.children.length === 1) {
          const next = Array.from(current.children).find(c => !shouldSkipElement(c));
          if (next && next.tagName.toLowerCase() === tagName && !hasDistinguishing(next)) {
            collapseCount++;
            current = next;
          } else {
            break;
          }
        }
        
        if (collapseCount >= 2) {
          shouldCollapse = true;
        }
      }
    }

    if (shouldCollapse && collapseCount >= 2) {
      // Collect all elements in the chain
      const chainElements = [node];
      let current = node;
      for (let i = 1; i < collapseCount && current.children.length > 0; i++) {
        const next = Array.from(current.children).find(c => !shouldSkipElement(c));
        if (next && next.tagName.toLowerCase() === tagName) {
          chainElements.push(next);
          current = next;
        } else {
          break;
        }
      }

      // Find the most specific selector for all elements in the chain
      const commonSelector = findCommonSelector(chainElements);
      const collapsedLine = `${indent}[${level}] (${collapseCount} containing ${commonSelector} elements)`;
      lines.push(collapsedLine);

      // The deepest element is the last one in the chain
      const deepest = chainElements[chainElements.length - 1];

      // Process children of the deepest element
      const deepestChildren = Array.from(deepest.children).filter(child =>
        !shouldSkipElement(child)
      );

      if (deepestChildren.length > 0) {
        const newIndent = indent + '│  ';
        deepestChildren.forEach((child, index) => {
          const isLast = index === deepestChildren.length - 1;
          const childIndent = isLast ? indent + '   ' : newIndent;
          const newParentChain = [...parentChain, { tagName, hasDistinguishingFeatures }];
          const childLines = serializeNode(child, childIndent, isSVG, newParentChain, level + 1);
          lines.push(...childLines);
        });
      }
    } else {
      // Normal serialization
      const selector = generateSelector(node);
      const textPreview = getTextPreview(node);
      
      // Calculate recursive child count
      const recursiveChildCount = countRecursiveChildren(node);
      
      // Build the line with level bracket
      let line = `${indent}[${level}] ${selector}`;
      
      // Add container info if > 20 recursive children
      if (recursiveChildCount > 20) {
        const innerTextInfo = getInnerTextInfo(node);
        const containerParts = [];
        
        if (innerTextInfo) {
          containerParts.push(`${innerTextInfo.length} chars of text`);
        }
        containerParts.push(`${recursiveChildCount} sub children`);
        
        line += ` {${containerParts.join(', ')}`;
        
        // Add text preview inside braces
        if (innerTextInfo && innerTextInfo.preview) {
          line += ` "[${innerTextInfo.preview}]"`;
        } else if (textPreview) {
          line += ` "${textPreview}"`;
        }
        
        line += '}';
      } else {
        // Add recursive child count if > 5 (but <= 20)
        if (recursiveChildCount > 5) {
          line += ` {${recursiveChildCount} sub children}`;
        }
        
        // Add text preview for non-containers
        if (textPreview) {
          line += ` "${textPreview}"`;
        }
      }
      
      lines.push(line);

      if (isSVG && tagName === 'svg') {
        return lines;
      }

      if (children.length === 0) return lines;

      const childIndent = indent + '│  ';
      const newParentChain = [...parentChain, { tagName, hasDistinguishingFeatures }];

      children.forEach((child, index) => {
        const isLast = index === children.length - 1;
        const newIndent = isLast ? indent + '   ' : childIndent;
        const childLines = serializeNode(child, newIndent, isSVG, newParentChain, level + 1);
        lines.push(...childLines);
      });
    }

    return lines;
  }

  analyzeClasses(root);

  const treeLines = serializeNode(root);
  
  return treeLines.join('\n');
}

if (typeof module !== 'undefined' && module.exports) {
  module.exports = serializeDOM;
}


console.log(serializeDOM());