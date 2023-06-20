const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const Dotenv = require('dotenv-webpack');
const { ProvidePlugin, NormalModuleReplacementPlugin } = require('webpack');

module.exports = {
  devtool: 'eval-cheap-source-map',
  entry: './src/bootstrap.ts',
  mode: 'development',
  module: {
    rules: [
      {
        test: /\.ts?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
    ],
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.js'],
    fallback: {
      path: require.resolve('path-browserify'),
      os: require.resolve('os-browserify/browser'),
      crypto: require.resolve('crypto-browserify'),
      stream: require.resolve('stream-browserify'),
      assert: require.resolve('assert/'),
      url: require.resolve('url'),
      fs: require.resolve('graceful-fs'),
      constants: require.resolve('constants-browserify'),
      http: require.resolve('stream-http'),
      https: require.resolve('https-browserify'),
      // timers: require.resolve('timers-browserify'),
    },
  },
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bootstrap.js',
    library: 'observer',
    libraryTarget: 'window',
  },
  devServer: {
    static: path.join(__dirname, 'dist'),
    compress: true,
    port: 4000,
  },
  plugins: [
    new Dotenv(),
    new CopyWebpackPlugin({ patterns: ['src/index.html'] }),
    new ProvidePlugin({
      Buffer: ['buffer', 'Buffer'],
      process: 'process/browser.js',
    }),
    // new NormalModuleReplacementPlugin(/node:/, (resource) => {
    //   const mod = resource.request.replace(/^node:/, '');
    //   switch (mod) {
    //     case 'buffer':
    //       resource.request = 'buffer';
    //       break;
    //     case 'stream':
    //       resource.request = 'readable-stream';
    //       break;
    //     case 'process':
    //       resource.request = 'process/browser';
    //       break;
    //     case 'fs':
    //       resource.request = 'graceful-fs';
    //       break;
    //     case 'path':
    //       resource.request = 'path-browserify';
    //       break;
    //     case 'util':
    //       resource.request = 'util';
    //       break;
    //     case 'http':
    //       resource.request = 'stream-http';
    //       break;
    //     case 'https':
    //       resource.request = 'https-browserify';
    //       break;
    //     case 'zlib':
    //       resource.request = 'browserify-zlib';
    //       break;
    //     case 'net':
    //       resource.request = 'net-browserify';
    //       break;
    //     case 'url':
    //       resource.request = 'url';
    //       break;
    //     case 'stream/web':
    //       resource.request = 'stream-browserify';
    //       break;
    //     default:
    //       throw new Error(`Not found ${mod}`);
    //   }
    // }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
};
