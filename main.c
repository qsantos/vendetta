#include <stdlib.h>
#include <SFML/Graphics.h>

typedef struct character character_t;
struct character
{
	float x;
	float y;
};

int main(void)
{
	sfVideoMode mode = {800, 600, 32};
	sfRenderWindow* window = sfRenderWindow_create(mode, "SFML window", sfResize | sfClose, NULL);
	if (window == NULL)
		exit(1);

	sfTexture* texture = sfTexture_createFromFile("character.png", NULL);
	if (texture == NULL)
		exit(1);

	sfSprite* sprite = sfSprite_create();
	sfSprite_setTexture(sprite, texture, sfTrue);

	sfIntRect rect = {24*1, 32*2, 24, 32};
	sfSprite_setTextureRect(sprite, rect);

	character_t player = {0, 0};

	sfClock* clock = sfClock_create();
	while (sfRenderWindow_isOpen(window))
	{
		float duration = sfTime_asSeconds(sfClock_getElapsedTime(clock));
		sfClock_restart(clock);

		sfEvent event;
		while (sfRenderWindow_pollEvent(window, &event))
		{
			if (event.type == sfEvtClosed)
				sfRenderWindow_close(window);
			else if (event.type == sfEvtKeyPressed)
			{
				sfKeyCode k = event.key.code;
				if (k == sfKeyEscape)
					sfRenderWindow_close(window);
			}
		}

		if (sfKeyboard_isKeyPressed(sfKeyUp))
			player.y -= 100 * duration;
		else if (sfKeyboard_isKeyPressed(sfKeyDown))
			player.y += 100 * duration;

		if (sfKeyboard_isKeyPressed(sfKeyLeft))
			player.x -= 100 * duration;
		else if (sfKeyboard_isKeyPressed(sfKeyRight))
			player.x += 100 * duration;

		sfRenderWindow_clear(window, sfBlack);

		sfVector2f pos = { player.x, player.y };
		sfSprite_setPosition(sprite, pos);
		sfRenderWindow_drawSprite(window, sprite, NULL);

		sfRenderWindow_display(window);
	}

	sfClock_destroy(clock);
	sfSprite_destroy(sprite);
	sfTexture_destroy(texture);
	sfRenderWindow_destroy(window);
	return 0;
}
