/**
 * FIGMA DATA DECODER - COMPLETE VERSION
 * 
 * A comprehensive JavaScript function to decode Figma data from HTML attributes.
 * This decoder properly extracts metadata and binary buffer data from Figma exports.
 * 
 * @param {string} htmlString - The HTML string containing data-metadata and data-buffer attributes
 * @returns {object} Decoded Figma data including metadata, buffer, and analysis
 */
function decodeFigmaData(htmlString) {
    try {
        // Extract metadata and buffer from HTML attributes
        const metadataMatch = htmlString.match(/data-metadata="([^"]+)"/);
        const bufferMatch = htmlString.match(/data-buffer="([^"]+)"/);
        
        if (!metadataMatch || !bufferMatch) {
            throw new Error('Could not find metadata or buffer attributes in HTML');
        }
        
        // Decode metadata - handle the specific format from the HTML
        console.log('Metadata string length:', metadataMatch[1].length);
        console.log('Metadata starts with:', metadataMatch[1].substring(0, 20));
        console.log('Metadata ends with:', metadataMatch[1].substring(metadataMatch[1].length - 20));
        
        // Extract metadata using the exact pattern from the HTML
        let metadataString = metadataMatch[1];
        const metadataBase64Match = metadataString.match(/<!--\(figmeta\)(.+?)\(\/figmeta\)-->/);
        if (metadataBase64Match) {
            metadataString = metadataBase64Match[1].trim();
        } else {
            // Manual extraction
            const startIdx = metadataString.indexOf('(figmeta)') + 9;
            const endIdx = metadataString.lastIndexOf('(/figmeta)');
            if (startIdx > 8 && endIdx > startIdx) {
                metadataString = metadataString.substring(startIdx, endIdx).trim();
            } else {
                throw new Error('Failed to extract metadata base64 content');
            }
        }
        
        console.log('Clean metadata length:', metadataString.length);
        console.log('Clean metadata:', metadataString);
        
        // Fix URL-safe base64 and add padding
        metadataString = metadataString.replace(/-/g, '+').replace(/_/g, '/');
        while (metadataString.length % 4 !== 0) {
            metadataString += '=';
        }
        
        const decodedMetadata = JSON.parse(atob(metadataString));
        
        // Decode buffer with proper HTML comment extraction
        const fullBufferString = bufferMatch[1];
        
        // Extract base64 content between HTML comment markers: <!--(figma)DATA(/figma)-->
        const base64Match = fullBufferString.match(/<!--\(figma\)(.+?)\(\/figma\)-->/);
        if (!base64Match) {
            throw new Error('Could not extract base64 content from buffer');
        }
        
        let cleanBuffer = base64Match[1].trim();
        
        // Fix URL-safe base64
        cleanBuffer = cleanBuffer.replace(/-/g, '+').replace(/_/g, '/');
        
        // Add padding if needed
        while (cleanBuffer.length % 4 !== 0) {
            cleanBuffer += '=';
        }
        
        // Decode buffer
        const decodedBuffer = atob(cleanBuffer);
        
        // Analyze the decoded buffer
        const bufferAnalysis = analyzeFigmaBuffer(decodedBuffer);
        
        return {
            success: true,
            metadata: decodedMetadata,
            buffer: decodedBuffer,
            bufferArray: Array.from(decodedBuffer).map(char => char.charCodeAt(0)),
            fileInfo: {
                fileKey: decodedMetadata.fileKey,
                pasteId: decodedMetadata.pasteID,
                dataType: decodedMetadata.dataType
            },
            bufferAnalysis: bufferAnalysis,
            raw: {
                metadataBase64: metadataMatch[1],
                bufferBase64: fullBufferString
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error.message,
            details: 'Failed to decode Figma data'
        };
    }
}

/**
 * Analyzes the decoded Figma buffer to extract useful information
 */
function analyzeFigmaBuffer(buffer) {
    const analysis = {
        length: buffer.length,
        format: 'unknown',
        header: '',
        compression: 'none',
        containsText: false,
        containsImages: false,
        structure: {}
    };
    
    // Check file format signature
    if (buffer.startsWith('fig-kiwie')) {
        analysis.format = 'figma-kiwie';
        analysis.header = 'fig-kiwie';
    } else if (buffer.startsWith('PNG')) {
        analysis.format = 'png-image';
        analysis.containsImages = true;
    } else if (buffer.startsWith('\x89PNG')) {
        analysis.format = 'png-image';
        analysis.containsImages = true;
    }
    
    // Check for compression
    if (buffer.startsWith('\x1F\x8B')) {
        analysis.compression = 'gzip';
    } else if (buffer.startsWith('PK')) {
        analysis.compression = 'zip';
    } else if (buffer.startsWith('\x78\x9C') || buffer.startsWith('\x78\x01')) {
        analysis.compression = 'zlib';
    }
    
    // Check for text content
    const textContent = buffer.match(/[a-zA-Z]/g);
    analysis.containsText = textContent && textContent.length > 10;
    
    // Extract some structural information
    if (analysis.format === 'figma-kiwie') {
        // Look for common Figma patterns in the binary data
        const patterns = {
            nodes: (buffer.match(/node/gi) || []).length,
            frames: (buffer.match(/frame/gi) || []).length,
            layers: (buffer.match(/layer/gi) || []).length,
            images: (buffer.match(/image/gi) || []).length,
            text: (buffer.match(/text/gi) || []).length
        };
        analysis.structure = patterns;
    }
    
    // Byte frequency analysis
    const byteFreq = {};
    for (let i = 0; i < Math.min(buffer.length, 1000); i++) {
        const byte = buffer.charCodeAt(i);
        byteFreq[byte] = (byteFreq[byte] || 0) + 1;
    }
    analysis.byteDistribution = byteFreq;
    analysis.entropy = Object.keys(byteFreq).length / 256;
    
    return analysis;
}

/**
 * Alternative function that takes metadata and buffer strings directly
 */
function decodeFigmaMetadataAndBuffer(metadataString, bufferString) {
    try {
        // Clean and decode metadata
        const cleanMetadata = metadataString
            .replace(/<!--\(figmeta\)-->/g, '')
            .replace(/<!--\/figmeta\)-->/g, '')
            .trim();
        
        const decodedMetadata = JSON.parse(atob(cleanMetadata));
        
        // Extract base64 content from buffer
        const base64Match = bufferString.match(/<!--\(figma\)(.+?)\(\/figma\)-->/);
        if (!base64Match) {
            throw new Error('Could not extract base64 content from buffer string');
        }
        
        let cleanBuffer = base64Match[1].trim();
        cleanBuffer = cleanBuffer.replace(/-/g, '+').replace(/_/g, '/');
        
        while (cleanBuffer.length % 4 !== 0) {
            cleanBuffer += '=';
        }
        
        const decodedBuffer = atob(cleanBuffer);
        
        return {
            success: true,
            metadata: decodedMetadata,
            buffer: decodedBuffer,
            fileInfo: {
                fileKey: decodedMetadata.fileKey,
                pasteId: decodedMetadata.pasteID,
                dataType: decodedMetadata.dataType
            },
            bufferAnalysis: analyzeFigmaBuffer(decodedBuffer)
        };
    } catch (error) {
        return {
            success: false,
            error: error.message
        };
    }
}

/**
 * Exports useful data from the Figma buffer
 */
function exportFigmaData(decodedData, outputDir = './output') {
    const fs = require('fs');
    const path = require('path');
    
    if (!decodedData.success) {
        throw new Error('Cannot export failed decode result');
    }
    
    // Create output directory
    if (!fs.existsSync(outputDir)) {
        fs.mkdirSync(outputDir, { recursive: true });
    }
    
    // Export metadata as JSON
    fs.writeFileSync(
        path.join(outputDir, 'metadata.json'), 
        JSON.stringify(decodedData.metadata, null, 2)
    );
    
    // Export raw buffer as binary
    fs.writeFileSync(
        path.join(outputDir, 'buffer.bin'), 
        decodedData.buffer
    );
    
    // Export analysis as JSON
    fs.writeFileSync(
        path.join(outputDir, 'analysis.json'), 
        JSON.stringify(decodedData.bufferAnalysis, null, 2)
    );
    
    // Export buffer as hex for debugging
    const hexData = Array.from(decodedData.buffer).map(byte => 
        byte.charCodeAt(0).toString(16).padStart(2, '0')
    ).join('');
    fs.writeFileSync(
        path.join(outputDir, 'buffer.hex'), 
        hexData
    );
    
    return {
        exported: ['metadata.json', 'buffer.bin', 'analysis.json', 'buffer.hex'],
        outputDir: outputDir
    };
}

// Export for Node.js environments
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        decodeFigmaData,
        decodeFigmaMetadataAndBuffer,
        analyzeFigmaBuffer,
        exportFigmaData
    };
}

// Example usage
if (require.main === module) {
    const fs = require('fs');
    
    console.log('=== FIGMA DATA DECODER - COMPLETE ANALYSIS ===\n');
    
    // Read and decode the HTML file
    const htmlContent = fs.readFileSync('figma.html', 'utf8');
    const result = decodeFigmaData(htmlContent);
    
    if (result.success) {
        console.log('✅ SUCCESSFUL DECODING!\n');
        
        console.log('📋 METADATA:');
        console.log(JSON.stringify(result.metadata, null, 2));
        
        console.log('\n📁 FILE INFO:');
        console.log(JSON.stringify(result.fileInfo, null, 2));
        
        console.log('\n🔍 BUFFER ANALYSIS:');
        console.log(JSON.stringify(result.bufferAnalysis, null, 2));
        
        // Export the data
        try {
            const exportResult = exportFigmaData(result, './figma-export');
            console.log('\n💾 EXPORTED FILES:');
            exportResult.exported.forEach(file => {
                console.log(`   - ${file}`);
            });
        } catch (error) {
            console.log('\n❌ Export failed:', error.message);
        }
        
    } else {
        console.log('❌ DECODING FAILED:', result.error);
    }
}