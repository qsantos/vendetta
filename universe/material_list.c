#include "material_list.h"

#include "../util.h"

void material_list_init(material_list_t* l)
{
	l->length  = 0;
	l->mat_ids = NULL;
	l->amounts = NULL;
}

void material_list_exit(material_list_t* l)
{
	free(l->amounts);
	free(l->mat_ids);
}

void material_list_push(material_list_t* l, int id, float a)
{
	int n = l->length++;
	l->mat_ids = CREALLOC(l->mat_ids, int,   l->length);
	l->amounts = CREALLOC(l->amounts, float, l->length);
	l->mat_ids[n] = id;
	l->amounts[n] = a;
}

int material_list_check(material_list_t* l, const float* amounts)
{
	for (int i = 0; i < l->length; i++)
	{
		float got  = amounts[l->mat_ids[i]];
		float need = l->amounts[i];
		if (got < need)
			return 0;
	}
	return 1;
}

float material_list_ratio(material_list_t* l, const float* amounts, float max_ratio)
{
	float ret = max_ratio;
	for (int i = 0; i < l->length; i++)
	{
		float got  = amounts[l->mat_ids[i]];
		float need = l->amounts[i];
		float r = got / need;
		if (ret < 0 || r < ret)
			ret = r;
	}
	return ret;
}

void material_list_apply(material_list_t* l, float* amounts, float ratio)
{
	for (int i = 0; i < l->length; i++)
		amounts[l->mat_ids[i]] += ratio * l->amounts[i];
}
