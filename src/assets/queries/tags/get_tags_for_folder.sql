select Tags.id, Tags.title
from Tags
         join Folders_Tags on Tags.id = Folders_Tags.tagId
where Folders_Tags.folderId = ?1;
