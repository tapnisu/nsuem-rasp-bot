use serde::Deserialize;

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupsList {
    pub data: Vec<GroupsListItem>,
    pub success: bool,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupsListItem {
    #[serde(rename = "GroupName")]
    pub group_name: String,
    #[serde(rename = "FacultyName")]
    pub faculty_name: String,
}

impl GroupsList {
    pub async fn fetch() -> reqwest::Result<Vec<GroupsListItem>> {
        let groups_list = reqwest::get("https://rasps.nsuem.ru/data/groups")
            .await?
            .json::<GroupsList>()
            .await?
            .data;

        Ok(groups_list)
    }
}
