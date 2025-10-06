#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';

// Simple icon generation using Node.js canvas-like approach
// Since we can't use external libraries easily, we'll use a simple approach
// with ImageMagick or sips if available

const sourceIcon = 'public/icon.png';
const iconDir = 'src-tauri/icons';

// Additional square icons for Microsoft Store
const squareIcons = [
  { size: 30, filename: 'Square30x30Logo.png' },
  { size: 44, filename: 'Square44x44Logo.png' },
  { size: 71, filename: 'Square71x71Logo.png' },
  { size: 89, filename: 'Square89x89Logo.png' },
  { size: 107, filename: 'Square107x107Logo.png' },
  { size: 142, filename: 'Square142x142Logo.png' },
  { size: 150, filename: 'Square150x150Logo.png' },
  { size: 284, filename: 'Square284x284Logo.png' },
  { size: 310, filename: 'Square310x310Logo.png' }
];

// Define required icon sizes
const iconSizes = [
  { size: 32, filename: '32x32.png' },
  { size: 128, filename: '128x128.png' },
  { size: 256, filename: '128x128@2x.png' },
  { size: 512, filename: 'icon.png' }
];

function generateIcons() {
  if (!fs.existsSync(sourceIcon)) {
    console.error(`Source icon ${sourceIcon} not found!`);
    process.exit(1);
  }

  // Create icons directory if it doesn't exist
  if (!fs.existsSync(iconDir)) {
    fs.mkdirSync(iconDir, { recursive: true });
  }

  // Check if ImageMagick is available
  try {
    execSync('magick -version', { stdio: 'ignore' });
    console.log('Using ImageMagick to generate icons...');
    
    iconSizes.forEach(({ size, filename }) => {
      const outputPath = path.join(iconDir, filename);
      execSync(`magick ${sourceIcon} -resize ${size}x${size} ${outputPath}`, { stdio: 'inherit' });
      console.log(`Generated ${filename} (${size}x${size})`);
    });

    // Generate square icons for Microsoft Store
    squareIcons.forEach(({ size, filename }) => {
      const outputPath = path.join(iconDir, filename);
      execSync(`magick ${sourceIcon} -resize ${size}x${size} ${outputPath}`, { stdio: 'inherit' });
      console.log(`Generated ${filename} (${size}x${size})`);
    });

    // Generate ICO file for Windows
    execSync(`magick ${sourceIcon} -resize 256x256 ${path.join(iconDir, 'icon.ico')}`, { stdio: 'inherit' });
    console.log('Generated icon.ico');

    // Generate ICNS file for macOS
    execSync(`magick ${sourceIcon} -resize 512x512 ${path.join(iconDir, 'icon.icns')}`, { stdio: 'inherit' });
    console.log('Generated icon.icns');

  } catch (e) {
    // Try using sips on macOS
    try {
      execSync('which sips', { stdio: 'ignore' });
      console.log('Using sips to generate icons...');
      
      iconSizes.forEach(({ size, filename }) => {
        const outputPath = path.join(iconDir, filename);
        execSync(`sips -z ${size} ${size} ${sourceIcon} --out ${outputPath}`, { stdio: 'inherit' });
        console.log(`Generated ${filename} (${size}x${size})`);
      });

      // Generate square icons for Microsoft Store using sips
      squareIcons.forEach(({ size, filename }) => {
        const outputPath = path.join(iconDir, filename);
        execSync(`sips -z ${size} ${size} ${sourceIcon} --out ${outputPath}`, { stdio: 'inherit' });
        console.log(`Generated ${filename} (${size}x${size})`);
      });

      // Generate ICNS using iconutil (macOS)
      try {
        const tempIconset = path.join(iconDir, 'icon.iconset');
        if (!fs.existsSync(tempIconset)) {
          fs.mkdirSync(tempIconset);
        }

        const icnsSizes = [16, 32, 128, 256, 512];
        icnsSizes.forEach(size => {
          execSync(`sips -z ${size} ${size} ${sourceIcon} --out ${path.join(tempIconset, `icon_${size}x${size}.png`)}`, { stdio: 'inherit' });
          execSync(`sips -z ${size*2} ${size*2} ${sourceIcon} --out ${path.join(tempIconset, `icon_${size}x${size}@2x.png`)}`, { stdio: 'inherit' });
        });

        execSync(`iconutil -c icns ${tempIconset}`, { stdio: 'inherit' });
        console.log('Generated icon.icns');
        
        // Clean up temp directory
        fs.rmSync(tempIconset, { recursive: true, force: true });
      } catch (e) {
        console.log('Could not generate ICNS file, but PNG icons were created successfully');
      }

    } catch (e) {
      console.error('Neither ImageMagick nor sips found. Please install ImageMagick or use macOS.');
      console.log('On macOS, sips is pre-installed.');
      console.log('On Ubuntu/Debian: sudo apt-get install imagemagick');
      console.log('On macOS with Homebrew: brew install imagemagick');
      process.exit(1);
    }
  }

  console.log('Icon generation complete!');
}

generateIcons();