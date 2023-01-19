FROM debian:11.6

RUN apt-get update && apt-get install wget libc6 -y
ENV BTC_VERSION=0.21.1
ENV ELECTRS_VERSION=0.4.1
RUN wget -nv https://bitcoincore.org/bin/bitcoin-core-${BTC_VERSION}/bitcoin-${BTC_VERSION}-x86_64-linux-gnu.tar.gz -O bitcoin-${BTC_VERSION}-x86_64-linux-gnu.tar.gz
RUN tar -xzf bitcoin-${BTC_VERSION}-x86_64-linux-gnu.tar.gz
RUN mv bitcoin-${BTC_VERSION} bitcoin
RUN wget -nv https://dlc-public-assets.s3.amazonaws.com/releases/electrs/electrs-${ELECTRS_VERSION}.tar.gz -O  electrs-${ELECTRS_VERSION}.tar.gz
RUN tar -xzf electrs-${ELECTRS_VERSION}.tar.gz
RUN chmod +x electrs
RUN mv electrs /usr/bin/electrs
ADD containers/bitcoind-electrs/conf/bitcoin.conf bitcoin/conf/bitcoin.conf
ADD containers/bitcoind-electrs/bin/start.sh start.sh
RUN chmod +x start.sh
