mkdir ./dist/.cert

openssl req -subj '/CN=' -new -newkey rsa:2048 -sha256 -days 365 -nodes -x509 -keyout server.key -out server.crt

mv server.crt ./dist/.cert
mv server.key ./dist/.cert
