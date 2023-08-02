export EDITOR=nano
export KUBE_EDITOR=nano
alias psql="PGPASSWORD=$PG_PASSWORD psql -U $PGUSER -h $PGHOST"
export DATABASE_URL="postgres://$PGUSER:$PG_PASSWORD@$PGHOST:5432/api"
