select f.id, f.name, f.parentId, group_concat(t.title)
from folders f
         join Folders_Tags ft on f.id = ft.folderId
         join tags t on t.id = ft.tagId
where t.title in (?1)
group by f.id;
