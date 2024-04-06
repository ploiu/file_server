rabbitmq-server &
sleep 10s
/opt/rabbitmq/sbin/rabbitmqctl add_user admin admin
/opt/rabbitmq/sbin/rabbitmqctl set_user_tags admin administrator
/opt/rabbitmq/sbin/rabbitmqctl set_permissions -p / admin ".*" ".*" ".*"
# rabbit dies when this script finishes, so this line is to keep the script running forever
tail -f /dev/null
