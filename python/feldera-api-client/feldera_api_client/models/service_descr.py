from typing import TYPE_CHECKING, Any, Dict, List, Type, TypeVar, Union

from attrs import define, field

if TYPE_CHECKING:
    from ..models.service_config_type_0 import ServiceConfigType0
    from ..models.service_config_type_1 import ServiceConfigType1


T = TypeVar("T", bound="ServiceDescr")


@define
class ServiceDescr:
    """Service descriptor.

    Attributes:
        config (Union['ServiceConfigType0', 'ServiceConfigType1']): A service's configuration.
        description (str):
        name (str):
        service_id (str): Unique service id.
    """

    config: Union["ServiceConfigType0", "ServiceConfigType1"]
    description: str
    name: str
    service_id: str
    additional_properties: Dict[str, Any] = field(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        from ..models.service_config_type_0 import ServiceConfigType0

        config: Dict[str, Any]

        if isinstance(self.config, ServiceConfigType0):
            config = self.config.to_dict()

        else:
            config = self.config.to_dict()

        description = self.description
        name = self.name
        service_id = self.service_id

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "config": config,
                "description": description,
                "name": name,
                "service_id": service_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.service_config_type_0 import ServiceConfigType0
        from ..models.service_config_type_1 import ServiceConfigType1

        d = src_dict.copy()

        def _parse_config(data: object) -> Union["ServiceConfigType0", "ServiceConfigType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_service_config_type_0 = ServiceConfigType0.from_dict(data)

                return componentsschemas_service_config_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_service_config_type_1 = ServiceConfigType1.from_dict(data)

            return componentsschemas_service_config_type_1

        config = _parse_config(d.pop("config"))

        description = d.pop("description")

        name = d.pop("name")

        service_id = d.pop("service_id")

        service_descr = cls(
            config=config,
            description=description,
            name=name,
            service_id=service_id,
        )

        service_descr.additional_properties = d
        return service_descr

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties