## Build instructions

__Step 1:__ Copy and paste the `.env.sample` file to `.env`.
__Step 2:__ Make any necessary changes in the variables in your `.env` file.

__Running locally:__ Run `./exec run`.

__Populating the database:__ Run `./exec populate`.

__Getting shell access to the database:__ Once your database is running (with `./exec run`), you can execute `psql` directly on the container to get shell access to it, using the following command: `./exec psql`.

__Dumping your database:__ Run `./exec dump`. You can also mention the file you want to dump the database to using `./exec dump <file-name>`. By default, it will dump the database to `dbdump.sql`.

__Clearing your database:__ Run `./exec cleardb`, `./exec clear-db`, `./exec resetdb` or `./exec reset-db`.

__Restoring your database from a file:__ Run `./exec restore <file-name>`.

__Accessing the Management UI of RabbitMQ:__ Once your rabbitmq instance is running (with `./exec run`), you can access the management UI of RabbitMQ using by going to `http://localhost:15672`. This port can be changed in the `.env` file.

__Accessing Vault:__ Same as RabbitMQ, but use port `8200` instead of `15672`.

__Accessing drone:__ Again, same thing, but this time, use port `8000`.

__Shutting down all containers:__ Run `./exec stop` or `./exec down`.