const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");
const { ProvidePlugin } = require("webpack");

module.exports = {
  entry: "./src/bootstrap.ts",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
    library: "observer",
    libraryTarget: "window",
  },
  mode: "development",
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
  },
  module: {
    rules: [
      {
        test: /\.ts?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
    ],
  },
  plugins: [
    new CopyWebpackPlugin({ patterns: ["src/index.html"] }),
    new ProvidePlugin({
      Buffer: ["buffer", "Buffer"],
      process: "process/browser",
    }),
  ],
  experiments: {
    asyncWebAssembly: true,
  },
};
