select id, title
from Tags
where lower(title) = lower(?1)
