export EDITOR=nano
alias psql="PGPASSWORD=$PG_PASSWORD psql -U $PGUSER -h $PGHOST"
alias start-frontend="cd /workspace/frontend && npm install && PORT=3001 npm start"
