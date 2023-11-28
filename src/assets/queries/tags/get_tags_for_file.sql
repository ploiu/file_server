select Tags.id, Tags.title
from Tags
         join Files_Tags on Tags.id = Files_Tags.tagId
where Files_Tags.fileRecordId = ?1;
