## Build instructions

__Step 1:__ Copy and paste the `.env.sample` file to `.env`.  
__Step 2:__ Make any necessary changes in the variables in your `.env` file.

__Running locally:__ Run `docker-compose run --rm api`.

__Populating the database:__ Slightly more complicated. Run `docker-compose run --rm api bash -c "source .env && cargo populate"`.

__Getting shell access to the database:__ Once your database is running (with `docker-compose run --rm api`), you can execute `psql` directly on the container to get shell access to it, using the following command: `docker-compose exec postgres psql -U <DATABASE_USERNAME> -d <DATABASE_NAME>`. These variables are the same as the ones you've configured in your `.env` file.

__Accessing the Management UI of RabbitMQ:__ Once your rabbitmq instance is running (with `docker-compose run --rm api`), you can access the management UI of RabbitMQ using by going to `http://localhost:15672`. This port can be changed in the `.env` file.

__Accessing Vault:__ Same as RabbitMQ, but use port `8200` instead of `15672`.

__Accessing drone:__ Again, same thing, but this time, use port `8000`.
